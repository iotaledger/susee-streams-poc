use axum::{
    routing,
    Router,
};

use super::payload_controller as controller;

pub fn routes() -> Router {
    Router::new()
        .route("/decode", routing::post(controller::decode))
}

pub const INFO: &str = "Decode payloads of a Node identified by its external_id";