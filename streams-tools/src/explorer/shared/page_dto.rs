use serde::{
    Deserialize,
    Serialize
};

use axum::{
    extract::Query,
    Json,
};

use crate::dao_helpers::Limit;
use crate::explorer::error::AppError;

#[derive(Serialize, Deserialize, Debug)]
pub struct Page<DataT> {
    pub data: Vec<DataT>,
    pub meta: PageMeta,
}

#[derive(Serialize, Deserialize, Debug)]
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
        let page_count_total = (items_count_total as f32 / paging_opt.limit as f32).ceil() as u32;
        PageMeta {
            page_indx: paging_opt.page,
            items_count: items_count as u32,
            items_limit: paging_opt.limit,
            page_count_total,
            items_count_total: items_count_total as u32,
            has_prev_page: paging_opt.page > 0,
            has_next_page: (page_count_total - 1) > paging_opt.page,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PagingOptions {
    pub page: u32,
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