use axum::{
    response::IntoResponse,
    extract::{
        Path,
        Query
    },
    Json,
    Extension,
};

use crate::{
    explorer::{
        error::AppError,
        app_state::AppState
    },
};

use super::{
    messages_dto::{
        MessageList,
        MessageConditions,
    },
    messages_service as service,
};

pub (crate) async fn index(
    Query(conditions): Query<MessageConditions>,
    Extension(state): Extension<AppState>,
) -> Result<Json<MessageList>, AppError> {
    if let Some(channel_id) = conditions.channel_id {
        service::index(&state, channel_id.as_str()).await.map(|resp| Json(resp))
    } else {
        Err(AppError::AtLeastOneConditionNeeded("'channel_id' is missing".to_string()))
    }
}

#[axum::debug_handler]
pub (crate) async fn get(Path(msg_id): Path<String>, Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ret_val = service::get(&state.messages, &state.user_store, msg_id.as_str()).await.map(|resp| Json(resp));
    ret_val.into_response()
}