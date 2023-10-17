use std::env::var;

pub fn get_port() -> String {
    var("PORT").unwrap_or("7000".to_string())
}

pub fn get_project() -> String {
    var("PROJECT").unwrap_or("random_pedersen".to_string())
}

pub fn get_service() -> String {
    var("SERVICE").unwrap_or("node".to_string())
}

pub fn get_node_id() -> String {
    var("NODE_ID").unwrap_or("1".to_string())
}

pub fn get_peer_count() -> String {
    var("NUM_NODES").unwrap_or("2".to_string())
}

pub fn get_mpc_threshold() -> String {
    var("MPC_THRESHOLD").unwrap_or("0.66".to_string())
}
