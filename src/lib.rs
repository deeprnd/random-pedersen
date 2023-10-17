mod cache;
mod routes;
mod utils;

use tracing::{event, Level};

use cache::state::create_state;
use routes::create_routes;
use utils::config::get_port;

pub async fn run() {
    event!(Level::DEBUG, "lib::run");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let app = create_routes(create_state());
    let address = format!("0.0.0.0:{}", get_port());

    event!(
        Level::DEBUG,
        "lib::run {}",
        utils::peers::get_node_address()
    );

    axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
