use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use tracing::{event, Level};
use uuid::Uuid;

use crate::{
    cache::state::{
        AppState, CommitmentForRandom, CommitmentForRandoms, CommittedRandom, CommittedRandomData,
    },
    utils::{
        commitment::{Commitment, Opening},
        config::get_node_id,
        errors::CacheError,
        peers::{get_commitment_from_peers, get_node_addresses},
        random::generate_random,
    },
};

// generates u32 random and saves as u64 so that we don't overflow during addition of co-commitment
async fn get_commitment_for_random() -> Result<(Commitment, Opening), StatusCode> {
    event!(
        Level::DEBUG,
        "routes::commitment::get_commitment_for_random"
    );

    let random = generate_random(4).map_err(|_error| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut arr = [0; 4];
    arr.copy_from_slice(&random[0..4]);
    let value = u32::from_le_bytes(arr);
    Ok(Commitment::new(value as u64))
}

// stores commitment in cache
async fn store_commitment(
    commitment_id: Uuid,
    committed_random: CommittedRandomData,
    state: Arc<AppState>,
) -> Result<(), CacheError> {
    event!(
        Level::DEBUG,
        "routes::commitment::store_commitment {}",
        commitment_id
    );

    state.cache.insert(commitment_id, committed_random).await;
    Ok(())
}

// returns addresses of all nodes
pub async fn get_nodes() -> Result<Json<Vec<String>>, StatusCode> {
    event!(Level::DEBUG, "routes::commitment::get_nodes");
    Ok(Json(get_node_addresses()))
}

// returns address of the node
pub async fn get_node_address(Path(node_id): Path<u16>) -> Result<Json<String>, StatusCode> {
    event!(Level::DEBUG, "routes::commitment::get_node_address");
    Ok(Json(crate::utils::peers::get_peer_address(node_id)))
}

// commits to newly generated random, sends the request to other nodes to co-commit and returns aggregated commitment with nodes ids
pub async fn commit_to_random(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CommitmentForRandoms>, StatusCode> {
    event!(Level::DEBUG, "routes::commitment::commit_to_random");

    let (commitment, opening) = get_commitment_for_random().await?;

    let commitment_id = Uuid::new_v4();
    event!(
        Level::DEBUG,
        "routes::commitment::commit_to_random::commitment_id: {}",
        commitment_id
    );

    store_commitment(
        commitment_id,
        CommittedRandomData {
            commitment: commitment.clone(),
            opening: opening,
        },
        state,
    )
    .await
    .map_err(|_error| StatusCode::INTERNAL_SERVER_ERROR)?;

    let commitment_for_random = CommitmentForRandom {
        node_id: get_node_id().parse::<u16>().unwrap(),
        commitment_id: commitment_id.as_u128(),
        commitment: commitment.to_bytes(),
    };

    let co_commitments = get_commitment_from_peers(commitment_for_random.clone(), None)
        .await
        .map_err(|_error| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut aggregated_commitment = commitment.clone();
    let mut node_ids = Vec::new();
    for co_commitment in co_commitments {
        let peer_commitment = Commitment::from_slice(&co_commitment.commitment)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        aggregated_commitment = aggregated_commitment + peer_commitment - commitment.clone(); // aggregate and remove dealer overcommitment
        node_ids.push(co_commitment.node_id);
    }

    node_ids.push(commitment_for_random.node_id); // adding dealer

    Ok(Json(CommitmentForRandoms {
        commitment_id: commitment_id.as_u128(),
        commitment: aggregated_commitment.to_bytes(),
        node_ids: node_ids,
        dealer_id: get_node_id().parse::<u16>().unwrap()
    }))
}

// co-commits to previous commitment and returns aggregated commitment with newly generated random
pub async fn co_commit_to_random(
    State(state): State<Arc<AppState>>,
    Json(previous_commitment): Json<CommitmentForRandom>,
) -> Result<Json<CommitmentForRandom>, StatusCode> {
    event!(Level::DEBUG, "routes::commitment::co_commit_to_random");

    let (commitment, opening) = get_commitment_for_random().await?;
    let commitment_bytes: &[u8] = &previous_commitment.commitment;
    let co_commitment = commitment
        + Commitment::from_slice(&commitment_bytes).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    store_commitment(
        Uuid::from_u128(previous_commitment.commitment_id),
        CommittedRandomData {
            commitment: co_commitment.clone(),
            opening: opening,
        },
        state,
    )
    .await
    .map_err(|_error| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CommitmentForRandom {
        node_id: get_node_id().parse::<u16>().unwrap(),
        commitment_id: previous_commitment.commitment_id,
        commitment: co_commitment.to_bytes(),
    }))
}

// reveals random opening for proofing and reconstruction
pub async fn reveal_random(
    State(state): State<Arc<AppState>>,
    Json(commitment): Json<CommitmentForRandom>,
) -> Result<Json<CommittedRandom>, StatusCode> {
    event!(Level::DEBUG, "routes::commitment::get_commitment");

    let key = Uuid::from_u128(commitment.commitment_id);
    let value = state.cache.get(&key).await.ok_or(StatusCode::NOT_FOUND)?;

    // invalidate cache
    state.cache.invalidate(&key).await;

    Ok(Json(CommittedRandom {
        commitment: value.commitment.to_bytes(),
        opening: value.opening.to_bytes(),
    }))
}

#[cfg(test)]
mod tests {

    use std::env::set_var;

    use crate::{
        cache::state::create_state,
        utils::{
            config::{get_mpc_threshold, get_peer_count},
            peers::{
                get_commit_to_random_endpoint, get_nodes_endpoint, get_peer_port,
                get_reveal_random_endpoint,
            },
        },
    };
    use axum::{routing::post, Router};
    use axum_test_helper::TestClient;
    use more_asserts::assert_ge;
    use reqwest::blocking::Client;

    use super::*;

    #[tokio::test]
    async fn test_co_commit_to_random() {
        let random1 = 123124;
        let (commitment1, opening1) = Commitment::new(random1);

        let node_1_commitment = CommitmentForRandom {
            node_id: 1,
            commitment_id: 123 as u128,
            commitment: commitment1.to_bytes(),
        };

        let state = create_state();
        let shared_state = Arc::new(state);
        let app = Router::new()
            .route("/co-commit-random", post(co_commit_to_random))
            .with_state(shared_state.clone());

        set_var("NODE_ID", "5");
        let commitment_str = serde_json::to_string(&node_1_commitment).unwrap();
        let res = TestClient::new(app)
            .post("/co-commit-random")
            .header("content-type", "application/json")
            .body(commitment_str)
            .send()
            .await;

        let co_commitment_response: CommitmentForRandom = res.json().await;
        let co_commitment_from_response =
            Commitment::from_slice(&co_commitment_response.commitment).unwrap();
        assert_eq!(co_commitment_response.node_id, 5);

        let key = Uuid::from_u128(node_1_commitment.commitment_id);
        let value = shared_state.cache.get(&key).await.unwrap();

        // validate cache
        assert_eq!(co_commitment_from_response, value.commitment);

        // validate co-commitment
        let commitment2_from_opening = Commitment::from_opening(&value.opening);
        let aggregated_commitment = commitment2_from_opening + commitment1;
        assert_eq!(aggregated_commitment, co_commitment_from_response);

        // validate aggregated random
        let aggregated_opening = value.opening + opening1;
        let commitment_from_aggregated_opening = Commitment::from_opening(&aggregated_opening);
        assert_eq!(
            co_commitment_from_response,
            commitment_from_aggregated_opening
        );
    }

    #[tokio::test]
    async fn test_reveal_random() {
        let random1 = 123124;
        let (commitment1, _opening1) = Commitment::new(random1);

        let node_1_commitment = CommitmentForRandom {
            node_id: 1,
            commitment_id: 123 as u128,
            commitment: commitment1.to_bytes(),
        };

        let state = create_state();
        let shared_state = Arc::new(state);
        let app1 = Router::new()
            .route("/co-commit-random", post(co_commit_to_random))
            .route("/reveal-random", post(reveal_random))
            .with_state(shared_state.clone());
        let client = TestClient::new(app1);

        set_var("NODE_ID", "5");
        let mut commitment_str = serde_json::to_string(&node_1_commitment).unwrap();
        let res1 = client
            .post("/co-commit-random")
            .header("content-type", "application/json")
            .body(commitment_str)
            .send()
            .await;

        let co_commitment_response: CommitmentForRandom = res1.json().await;
        let key = Uuid::from_u128(co_commitment_response.commitment_id);
        assert_eq!(shared_state.cache.contains_key(&key), true); // should exist

        commitment_str = serde_json::to_string(&co_commitment_response).unwrap();
        let res2 = client
            .post("/reveal-random")
            .header("content-type", "application/json")
            .body(commitment_str)
            .send()
            .await;

        assert_eq!(shared_state.cache.contains_key(&key), false); // shouldn't exist
        let random2_response: CommittedRandom = res2.json().await;

        // validate cache and response
        let co_commitment_from_response =
            Commitment::from_slice(&co_commitment_response.commitment).unwrap();
        let co_commitment_from_cache =
            Commitment::from_slice(&random2_response.commitment).unwrap();
        assert_eq!(co_commitment_from_response, co_commitment_from_cache);

        // validate openings
        let opening2_from_response = Opening::from_slice(&random2_response.opening).unwrap();
        let commitment2_from_opening = Commitment::from_opening(&opening2_from_response);
        let aggregated_commitment = commitment2_from_opening + commitment1;
        assert_eq!(aggregated_commitment, co_commitment_from_response);
    }

    fn get_peer_address_mock(index: u16) -> String {
        format!("http://127.0.0.1:{}", get_peer_port_mock(index))
    }

    fn get_peer_port_mock(node_number: u16) -> u16 {
        get_peer_port(node_number) + node_number
    }

    #[test]
    #[ignore]
    fn test_e2e_get_nodes() {
        set_var("NUM_NODES", "3");

        let mut responses = Vec::new();
        let num_nodes = get_peer_count().parse::<u16>().unwrap();
        let client = Client::new();

        let mut response_object: Vec<String> = Vec::new();

        for index in 1..num_nodes + 1 {
            let address = format!("{}{}", get_peer_address_mock(index), get_nodes_endpoint());
            let response = client.get(address).send().unwrap();

            response_object = response.json::<Vec<String>>().unwrap();
            response_object.sort();
            responses.push(response_object.clone());
        }

        for response in &responses {
            let matching = response
                .iter()
                .zip(&response_object)
                .filter(|&(a, b)| a == b)
                .count();
            assert_eq!(matching, num_nodes as usize);
        }
    }

    #[test]
    #[ignore]
    fn test_e2e_commit_reveal() {
        set_var("NUM_NODES", "2");

        let mut responses = Vec::new();
        let num_nodes = get_peer_count().parse::<u16>().unwrap();
        let client = Client::new();

        let address = format!(
            "{}{}",
            get_peer_address_mock(1),
            get_commit_to_random_endpoint()
        );
        let response = client.post(address).send().unwrap();

        let response_object = response.json::<CommitmentForRandoms>().unwrap();

        let threshold = (get_mpc_threshold().parse::<f32>().unwrap() * num_nodes as f32).floor();
        assert_ge!(response_object.node_ids.len() as f32, threshold);

        let mut aggr_value = 0;
        let mut aggr_opening: Option<Opening> = None;
        let mut aggr_commitment: Option<Commitment> = None;
        let mut dealer_commitment: Option<Commitment> = None;

        for node_id in &response_object.node_ids {
            let node_address = format!(
                "{}{}",
                get_peer_address_mock(*node_id),
                get_reveal_random_endpoint()
            );

            let node_response = client
                .post(node_address)
                .json(&CommitmentForRandom {
                    commitment_id: response_object.commitment_id,
                    commitment: Vec::new(),
                    node_id: 0,
                })
                .send()
                .unwrap();

            let response_node_object = node_response.json::<CommittedRandom>().unwrap();
            responses.push(response_node_object.clone());

            let node_opening = Opening::from_slice(&response_node_object.opening).unwrap();
            let node_commitment = Commitment::from_slice(&response_node_object.commitment).unwrap();
            aggr_value += node_opening.value;

            if aggr_opening.is_some() {
                aggr_opening = Some(aggr_opening.unwrap() + node_opening);
            } else {
                aggr_opening = Some(node_opening);
            }

            if aggr_commitment.is_some() {
                aggr_commitment = Some(aggr_commitment.unwrap() + node_commitment.clone());
            } else {
                aggr_commitment = Some(node_commitment.clone());
            }

            if *node_id == response_object.dealer_id {
                dealer_commitment = Some(node_commitment);
            }
        }

        assert_eq!(aggr_value, aggr_opening.clone().unwrap().value);
        assert_eq!(
            Commitment::from_opening(&aggr_opening.unwrap()),
            Commitment::from_slice(&response_object.commitment).unwrap()
        );

        // verifying dealer overcommitment
        for _n in 1..response_object.node_ids.len() {
            aggr_commitment = Some(aggr_commitment.unwrap() - dealer_commitment.clone().unwrap());
        }

        assert_eq!(
            aggr_commitment.unwrap(),
            Commitment::from_slice(&response_object.commitment).unwrap()
        );
    }
}
