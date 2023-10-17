use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::utils::commitment::{Commitment, Opening};

#[allow(dead_code)]
#[derive(Clone)]
pub struct CommittedRandomData {
    pub commitment: Commitment,
    pub opening: Opening,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommittedRandom {
    pub commitment: Vec<u8>,
    pub opening: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommitmentForRandom {
    pub node_id: u16,
    pub commitment_id: u128,
    pub commitment: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CommitmentForRandoms {
    pub commitment_id: u128,
    pub commitment: Vec<u8>,
    pub node_ids: Vec<u16>,
    pub dealer_id: u16
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct AppState {
    pub cache: Cache<Uuid, CommittedRandomData>,
}

pub fn create_state() -> AppState {
    let cache = Cache::builder()
        // Max 10,000 entries
        .max_capacity(10_000)
        // Time to live (TTL): 30 minutes
        .time_to_live(Duration::from_secs(30 * 60))
        // Time to idle (TTI):  5 minutes
        .time_to_idle(Duration::from_secs(5 * 60))
        // Create the cache.
        .build();
    AppState { cache }
}
