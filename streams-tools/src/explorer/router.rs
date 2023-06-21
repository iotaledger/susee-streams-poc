use axum::{
    routing::{get},
    Router,
};

use super::{
    nodes,
    messages,
};

pub async fn route_info() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "routes": ["/", "/nodes", "/messages"],
        "routes_info": {
            "/" : "this route",
            "/nodes": nodes::INFO,
            "/messages": messages::INFO,
        }
    }))
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(route_info))
        .nest("/nodes", nodes::routes())
        .nest("/messages", messages::routes())
}