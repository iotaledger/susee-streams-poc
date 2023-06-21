use axum::{
    routing::get,
    Router,
};

use super::nodes_controller as controller;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(controller::index))
        .route("/:channel_id", get(controller::get))
}

pub const INFO: &str = "Search for nodes and view node details";