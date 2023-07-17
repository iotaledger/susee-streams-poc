use hyper::{
    Body,
    http::{
        Response,
        Result,
        status::StatusCode,
    }
};
use crate::http::{
    DispatchScope,
    http_tools::DispatchedRequestParts,
};

use iota_streams::core::async_trait;

#[async_trait(?Send)]
pub trait ServerProcessFinally {
    async fn process(&self, ret_val: Response<Body>, req_parts: &DispatchedRequestParts, scope: &dyn DispatchScope) -> Result<Response<Body>>;
}

pub fn get_final_http_status(original_status: &StatusCode, use_compressed_fn_hint: bool) -> StatusCode {
    if !use_compressed_fn_hint {
        original_status.clone()
    } else {
        match original_status {
            &StatusCode::OK => StatusCode::ALREADY_REPORTED,
            _ => *original_status,
        }
    }
}
