use std::cmp;

use serde::{
    Deserialize,
    Serialize
};

use axum::{
    extract::Query,
    Json,
};

use utoipa::{
    IntoParams,
    ToSchema
};

use crate::{
    dao_helpers::Limit,
    explorer::error::AppError,
};

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Page<DataT> {
    pub data: Vec<DataT>,
    pub meta: PageMeta,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct PageMeta {
    pub page_indx: u32,
    pub items_count: u32,
    pub items_limit: u32,
    pub page_count_total: u32,
    pub items_count_total: u32,
    pub has_prev_page: bool,
    pub has_next_page: bool,
}

impl PageMeta {
    pub fn new(paging_opt: PagingOptions, items_count: usize, items_count_total: usize) -> PageMeta {
        let page_count_total: u32 = if paging_opt.limit > 0 {
            (items_count_total as f32 / paging_opt.limit as f32).ceil() as u32
        } else {
            0
        };
        let has_next_page = if page_count_total > 1 {
            (page_count_total - 1) > paging_opt.page
        } else {
            false
        };
        let page_indx = cmp::min(paging_opt.page, page_count_total);
        PageMeta {
            page_indx,
            items_count: items_count as u32,
            items_limit: paging_opt.limit,
            page_count_total,
            items_count_total: items_count_total as u32,
            has_prev_page: page_indx > 0,
            has_next_page,
        }
    }
}

/// Control pagination of result list
#[derive(Serialize, Deserialize, Debug, Clone, IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
pub struct PagingOptions {
    /// Which page to get. Index range is [0 ...]
    #[param(default=0)]
    pub page: u32,
    /// Maximum number of items per page
    #[param(default=10)]
    pub limit: u32,
}

impl Default for PagingOptions {
    fn default() -> Self {
        PagingOptions {
            page: 0,
            limit: 10,
        }
    }
}

impl From<PagingOptions> for Limit {
    fn from(value: PagingOptions) -> Self {
        Limit {
            offset: (value.page * value.limit) as usize,
            limit: value.limit  as usize,
        }
    }
}

pub fn get_paging(optional_paging: Option<Query<PagingOptions>>) -> Option<PagingOptions> {
    optional_paging
        .map(|Query(paging)| {paging})
        .or(Some(PagingOptions::default()))
}

pub fn wrap_with_page_meta_and_json_serialize<DataT>(data: Vec<DataT>, paging: PagingOptions, items_cnt_total: usize) -> Result<Json<Page<DataT>>, AppError> {
    let data_len = data.len();
    let ret_val = Page::<DataT> {
        data,
        meta: PageMeta::new(paging, data_len, items_cnt_total)
    };
    Ok(Json(ret_val))
}