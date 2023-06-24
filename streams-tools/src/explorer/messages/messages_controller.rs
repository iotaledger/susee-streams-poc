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
        shared::{
            Page,
            PagingOptions,
            get_paging,
            wrap_with_page_meta_and_json_serialize,
        },
    },
};

use super::{
    messages_dto::{
        Message,
        MessageId,
        MessageConditions,
    },
    messages_service as service,
};

/// List messages of a node
///
/// List messages of a Streams channel of a specific node.
#[utoipa::path(
    get,
    operation_id = "messages_index",
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
    operation_id = "messages_get",
    path = "/messages/{message_id}",
    responses(
        (status = 200, description = "Successfully responded requested message", body = [Message]),
        (status = 404, description = "Message with specified msg_id does not exist")
    ),
    params(
        MessageId,
    )
)]
pub (crate) async fn get(Path(id): Path<MessageId>, Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ret_val = service::get(&state.messages, &state.user_store, id.message_id.as_str()).await.map(|resp| Json(resp));
    ret_val.into_response()
}