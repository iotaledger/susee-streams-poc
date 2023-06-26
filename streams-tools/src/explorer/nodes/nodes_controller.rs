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
        app_state::AppState,
        shared::{
            Page,
            PagingOptions,
            get_paging,
            wrap_with_page_meta_and_json_serialize,
        },
    }
};

use super::{
    nodes_dto::{
        Node,
        NodeConditions,
        ChannelId,
    },
    nodes_service as service,
};

/// Search for Nodes
///
/// Search for Nodes by Streams channel id, Name or external id
#[utoipa::path(
    get,
    operation_id = "nodes_index",
    path = "/nodes",
    responses(
        (status = 200, description = "Successfully responded with list of Nodes")
    ),
    params(
        NodeConditions,
        PagingOptions,
    )
)]
pub (crate) async fn index(
    Query(conditions): Query<NodeConditions>,
    optional_paging: Option<Query<PagingOptions>>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Page<Node>>, AppError> {
    let paging = get_paging(optional_paging);
    let (ret_val, items_cnt_total) = service::index(
        conditions,
        &state.user_store,
        paging.clone()
    )?;
    wrap_with_page_meta_and_json_serialize(ret_val, paging.unwrap(), items_cnt_total)
}

/// Get a specific Node
#[utoipa::path(
    get,
    operation_id = "nodes_get",
    path = "/nodes/{channel_id}",
    responses(
        (status = 200, description = "Successfully responded requested node", body = [Node]),
        (status = 400, description = "Node with specified channel_id does not exist")
    ),
    params(
        ChannelId,
    )
)]
pub (crate) async fn get(Path(id): Path<ChannelId>, Extension(state): Extension<AppState>) -> Result<Json<Node>, AppError> {
    service::get(&id.channel_id, &state.user_store).map(|resp| Json(resp))
}

/// Update a specific Node
///
/// Update the Node specified by the 'channel_id' path parameter.
/// The 'channel_id' field of the Node provided in the request body will be ignored.
/// Only Node fields 'external_id' and 'name' will be updated.
#[utoipa::path(
    put,
    operation_id = "nodes_put",
    path = "/nodes/{channel_id}",
    request_body = Node,
    responses(
        (status = 200, description = "Successfully updated requested node", body = [Node]),
        (status = 400, description = "Node with specified channel_id does not exist")
    ),
    params(
        ChannelId,
    )
)]
pub (crate) async fn put(Path(id): Path<ChannelId>, Extension(state): Extension<AppState>, Json(node): Json<Node>) -> Result<Json<Node>, AppError> {
    service::put(&id.channel_id, &state.user_store, node).map(|resp| Json(resp))
}
