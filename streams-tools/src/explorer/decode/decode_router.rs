use axum::{
    routing,
    Router,
};

use super::decode_controller as controller;

pub fn routes() -> Router {
    Router::new()
        .route("/", routing::post(controller::decode))
}

pub const INFO: &str = "Decode payloads of a Node identified by its external_id";