use axum::http::StatusCode;
use moka::future::Cache;
use uuid::Uuid;

use crate::utils::random::generate_random;

pub async fn get_committed_random() -> Result<Vec<u8>, StatusCode> {
    generate_random(32).map_err(|_error| StatusCode::INTERNAL_SERVER_ERROR)
}