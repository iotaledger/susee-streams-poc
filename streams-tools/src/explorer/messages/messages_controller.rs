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
        app_state::AppState,
        shared::page_dto::{
            PagingOptions,
            get_paging,
            wrap_with_page_meta_and_json_serialize,
        },
    },
};

use super::{
    messages_dto::{
        MessageConditions,
    },
    messages_service as service,
};
use crate::explorer::messages::messages_dto::Message;
use crate::explorer::shared::page_dto::Page;

/// List messages
///
/// List messages of a specific node.
/// Filtering the messages by the nodes channel id is obligatory.
#[utoipa::path(
    get,
    path = "/messages",
    responses(
        (status = 200, description = "Successfully responded with list of Messages"),
        (status = 400, description = "Channel with specified channel-id does not exist"),
    ),
    params(
        MessageConditions,
        PagingOptions,
    )
)]
pub (crate) async fn index(
    Query(conditions): Query<MessageConditions>,
    optional_paging: Option<Query<PagingOptions>>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Page<Message>>, AppError> {
    if let Some(channel_id) = conditions.channel_id {
        let paging = get_paging(optional_paging);
        let (ret_val, items_cnt_total) = service::index(&state, channel_id.as_str(), paging.clone()).await?;
        wrap_with_page_meta_and_json_serialize(ret_val, paging.unwrap(), items_cnt_total)
    } else {
        Err(AppError::AtLeastOneConditionNeeded("'channel_id' is missing".to_string()))
    }
}

#[utoipa::path(
    get,
    path = "/messages/{msg_id}",
    responses(
        (status = 200, description = "Successfully responded requested message", body = [Message]),
        (status = 404, description = "Message with specified id not found")
    ),
    params(
        ("msg_id" = i32, Path, description = "Message id of the requested message (includes the channel id)"),
    )
)]
pub (crate) async fn get(Path(msg_id): Path<String>, Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ret_val = service::get(&state.messages, &state.user_store, msg_id.as_str()).await.map(|resp| Json(resp));
    ret_val.into_response()
}