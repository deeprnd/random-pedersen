use axum::http::Method;
use tower_http::cors::{Any, CorsLayer};

// needs to be extended to allow requests from peers only for co-commitment
pub fn get_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
}
