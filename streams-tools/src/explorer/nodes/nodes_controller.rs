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
        shared::page_dto::{
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
    },
    nodes_service as service,
};
use crate::explorer::shared::page_dto::Page;

#[utoipa::path(
    get,
    path = "/nodes",
    responses(
        (status = 200, description = "Successfully responded with list of Nodes")
    ),
    params(
        NodeConditions,
        ("page" = u32, Path, description = "Which page to get. Index range is [0 ...]"),
        ("limit" = u32, Path, description = "Maximum number of items per page"),
    )
)]
pub (crate) async fn index(
    Query(conditions): Query<NodeConditions>,
    optional_paging: Option<Query<PagingOptions>>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Page<Node>>, AppError> {
    let paging = get_paging(optional_paging);
    let (ret_val, items_cnt_total) = {
        if let Some(channel_id_start) = conditions.channel_id_start {
            service::index(channel_id_start.as_str(), &state.user_store, paging.clone())
        } else {
            service::index("", &state.user_store, paging.clone())
        }
    }?;
    wrap_with_page_meta_and_json_serialize(ret_val, paging.unwrap(), items_cnt_total)
}

#[utoipa::path(
    get,
    path = "/nodes/{channel_id}",
    responses(
        (status = 200, description = "Successfully responded requested node", body = [Node]),
        (status = 404, description = "Node with specified id not found")
    ),
    params(
        ("channel_id" = i32, Path, description = "Streams channel id of the requested node"),
    )
)]
pub (crate) async fn get(Path(channel_id): Path<String>, Extension(state): Extension<AppState>) -> Result<Json<Node>, AppError> {
    service::get(&channel_id, &state.user_store).map(|resp| Json(resp))
}
