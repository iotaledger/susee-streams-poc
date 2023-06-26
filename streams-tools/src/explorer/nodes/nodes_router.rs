use axum::{
    routing,
    Router,
};

use super::nodes_controller as controller;

pub fn routes() -> Router {
    Router::new()
        .route("/", routing::get(controller::index))
        .route("/:channel_id", routing::get(controller::get))
        .route("/:channel_id", routing::put(controller::put))
}

pub const INFO: &str = "Search for nodes and view + update node details";