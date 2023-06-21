use axum::{
    extract::{
        Path,
        Query
    },
    Json,
    Extension
};

use crate::{
    explorer::{
        error::AppError,
        app_state::AppState
    }
};

use super::{
    nodes_dto::{
        Node,
        NodeList,
        NodeConditions,
    },
    nodes_service as service,
};

pub (crate) async fn index(
    Query(conditions): Query<NodeConditions>,
    Extension(state): Extension<AppState>,
) -> Result<Json<NodeList>, AppError> {
    {
        if let Some(channel_id_start) = conditions.channel_id_start {
            service::index(channel_id_start.as_str(), &state.user_store)
        } else {
            service::index("", &state.user_store)
        }
    }
    .map(|resp| Json(resp))
}

pub (crate) async fn get(Path(channel_id): Path<String>, Extension(state): Extension<AppState>) -> Result<Json<Node>, AppError> {
    service::get(&channel_id, &state.user_store).map(|resp| Json(resp))
}
