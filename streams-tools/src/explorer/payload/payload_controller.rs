use axum::{
    extract::{
        Query
    },
    Json,
    Extension
};

use hyper::body::Bytes;

use crate::explorer::{
    error::AppError,
    app_state::AppState,
    messages::Message,
};

use super::{
    payload_service as service,
    payload_dto::DecodeQueryParams,
};

/// Decode a payload
///
/// Decode a payload that has been send by a Node that is identified by its 'external_id'.
/// The payload needs to be provided in the request body.
#[utoipa::path(
    post,
    operation_id = "decode_post",
    path = "/payload/decode",
    request_body = [u8],
    responses(
        (status = 200, description = "Successfully decoded uploaded payload", body = Vec<u8>),
        (status = 400, description = "The uploaded payload could not be parsed because it is syntactically not correct"),
        (status = 404, description = "A Node with the specified external_id does not exist"),
    ),
    params(DecodeQueryParams)
)]
pub (crate) async fn decode(Query(params): Query<DecodeQueryParams>, Extension(state): Extension<AppState>, body_bytes: Bytes) -> Result<Json<Message>, AppError> {
    service::decode(
        &params.external_id,
        &state.user_store,
        &state.messages,
        body_bytes.to_vec()
    )
    .await.map(|resp| Json(resp))
}
