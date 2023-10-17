use futures::prelude::*;
use futures::stream::FuturesUnordered;
use reqwest::Client;
use tracing::{event, Level};

use crate::{
    cache::state::CommitmentForRandom,
    utils::{config::get_peer_count, errors::CommitmentGenerationError},
};

use super::config::{get_mpc_threshold, get_node_id, get_port, get_project, get_service};

pub fn get_commit_to_random_endpoint() -> String {
    "/commit-random".to_owned()
}

pub fn get_co_commit_to_random_endpoint() -> String {
    "/co-commit-random".to_owned()
}

pub fn get_nodes_endpoint() -> String {
    "/nodes".to_owned()
}

pub fn get_reveal_random_endpoint() -> String {
    "/reveal-random".to_owned()
}

pub fn get_node_address() -> String {
    get_peer_address(get_node_id().parse::<u16>().unwrap())
}

pub fn get_peer_hostname(index: u16) -> String {
    format!("{}_{}_{}", get_project(), get_service(), index)
}

pub fn get_peer_address(index: u16) -> String {
    format!(
        "http://{}:{}",
        get_peer_hostname(index),
        get_peer_port(index)
    )
}

fn get_peer_endpoint(index: u16) -> String {
    format!("{}/co-commit-random", get_peer_address(index))
}

#[allow(unused_variables)]
pub fn get_peer_port(node_number: u16) -> u16 {
    get_port().parse::<u16>().unwrap()
}

pub fn get_node_addresses() -> Vec<String> {
    let mut peers: Vec<String> = Vec::new();
    let num_nodes = get_peer_count().parse::<u16>().unwrap();

    for index in 1..num_nodes + 1 {
        peers.push(get_peer_address(index));
    }

    peers
}

fn get_peer_addresses(node_id: u16, num_nodes: u16) -> Vec<String> {
    let mut peers: Vec<String> = Vec::new();

    for index in 1..num_nodes + 1 {
        // Skip generating address for the current node (node_id).
        if index == node_id {
            continue;
        }

        let peer_address = get_peer_endpoint(index);
        peers.push(peer_address);
    }

    peers
}

// sends commitment to peer
pub async fn send_commitment_request(
    address: &str,
    commitment_for_random: CommitmentForRandom,
    http_client: Option<Client>,
) -> Result<CommitmentForRandom, reqwest::Error> {
    event!(
        Level::DEBUG,
        "utils::peer::send_commitment_request {}",
        address
    );
    let client = match http_client {
        Some(value) => value,
        None => Client::new(),
    };

    let response = client
        .post(address)
        .json(&commitment_for_random)
        .send()
        .await?
        .json::<CommitmentForRandom>()
        .await?;

    Ok(response)
}

pub async fn get_commitment_from_peers(
    commitment_for_random: CommitmentForRandom,
    http_client: Option<Client>,
) -> Result<Vec<CommitmentForRandom>, CommitmentGenerationError> {
    event!(Level::DEBUG, "utils::peer::get_commitment_from_peers");

    let num_nodes = get_peer_count().parse::<u16>().unwrap();
    let initial_peers = get_peer_addresses(get_node_id().parse::<u16>().unwrap(), num_nodes);
    let threshold = (get_mpc_threshold().parse::<f32>().unwrap() * num_nodes as f32).floor(); // 2/3 of num_nodes

    let mut futures = FuturesUnordered::new();

    for address in initial_peers {
        let commitment = commitment_for_random.clone();
        let http_client_clone = http_client.clone();
        let fut = async move {
            let response = send_commitment_request(&address, commitment, http_client_clone).await;
            if response.is_err() {
                let error = response.err().unwrap() as reqwest::Error;
                event!(
                    Level::ERROR,
                    "utils::peer::get_commitment_from_peers::error {:?}",
                    error
                );
                Err(error.without_url())
            } else {
                response
            }
        };

        futures.push(tokio::spawn(fut));
    }

    // Wait for all futures to complete and collect the responses.
    let mut responses: Vec<CommitmentForRandom> = Vec::new();
    while let Some(result) = futures.next().await {
        match result {
            Ok(join_response) => match join_response {
                Ok(commitment_response) => {
                    responses.push(commitment_response);
                }
                Err(err) => {
                    event!(
                        Level::ERROR,
                        "utils::peer::get_commitment_from_peers::reading http futures {:?}",
                        err
                    );
                }
            },
            Err(err) => {
                event!(
                    Level::ERROR,
                    "utils::peer::get_commitment_from_peers::reading joined futures {:?}",
                    err
                );
            }
        }
    }

    event!(
        Level::DEBUG,
        "utils::peer::get_commitment_from_peers::futures_count {}",
        responses.len()
    );

    if (responses.len() as f32) >= threshold {
        Ok(responses)
    } else {
        Err(CommitmentGenerationError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_peer_addresses() {
        let node_id = 2;
        let num_nodes = 5;

        let initial_peers = get_peer_addresses(node_id, num_nodes);

        // Ensure that the generated addresses do not contain the address for the current node.
        assert!(!initial_peers.contains(&get_peer_endpoint(node_id)));
    }

    #[test]
    fn test_get_peer_addresses_no_duplicate() {
        let node_id = 2; // Example node ID
        let num_nodes = 5; // Example number of nodes

        let initial_peers = get_peer_addresses(node_id, num_nodes);

        // Ensure that the generated addresses do not contain duplicates
        assert_eq!(
            initial_peers.len(),
            initial_peers
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    #[test]
    fn test_get_peer_addresses_count() {
        let node_id = 2; // Example node ID
        let num_nodes = 5; // Example number of nodes

        let initial_peers = get_peer_addresses(node_id, num_nodes);

        // Ensure that the number of received addresses is num_nodes - 1
        assert_eq!(initial_peers.len(), (num_nodes - 1) as usize);
    }
}
