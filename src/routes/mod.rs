mod commitment;
mod cors;

use axum::{
    routing::{get, post},
    Router,
};
use commitment::{
    co_commit_to_random, commit_to_random, get_node_address, get_nodes, reveal_random,
};
use std::sync::Arc;
use tracing::{event, Level};

use crate::{
    cache::state::AppState,
    utils::peers::{
        get_co_commit_to_random_endpoint, get_commit_to_random_endpoint, get_nodes_endpoint,
        get_reveal_random_endpoint,
    },
};

pub fn create_routes(state: AppState) -> Router {
    event!(Level::DEBUG, "routes::mod::create_routes");

    Router::new()
        .layer(cors::get_cors())
        .route(&get_commit_to_random_endpoint(), post(commit_to_random))
        .route(
            &get_co_commit_to_random_endpoint(),
            post(co_commit_to_random),
        )
        .route(&get_reveal_random_endpoint(), post(reveal_random))
        .route(&get_nodes_endpoint(), get(get_nodes))
        .route("/node/:node_id", get(get_node_address))
        .with_state(Arc::new(state))
}
