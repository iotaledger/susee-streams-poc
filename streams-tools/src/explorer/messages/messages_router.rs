use axum::{
    routing::{get},
    Router,
};

use super::messages_controller as controller;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(controller::index))
        .route("/:message_id", get(controller::get))
}

pub const INFO: &str = "Search for messages of a specific node in the tangle and view message details";