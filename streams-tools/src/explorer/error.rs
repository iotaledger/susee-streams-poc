use std::result;

use hyper::http::StatusCode;

use serde_json::json;

use axum::{
    response::IntoResponse,
    Json
};

use utoipa::{
    ToSchema
};

#[derive(Debug, ToSchema)]
pub enum AppError {
    InternalServerError(String),
    AtLeastOneConditionNeeded(String),
    ChannelDoesNotExist(String),
//    Generic(StatusCode),
    GenericWithMessage(StatusCode, String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("an internal server error occurred: {}", msg),
            ),
            Self::AtLeastOneConditionNeeded(msg) => (
                StatusCode::BAD_REQUEST,
                format!("at least one condition query parameter is needed: {}", msg),
            ),
            Self::ChannelDoesNotExist(channel_id) => (
                StatusCode::BAD_REQUEST,
                format!("The channel with channel-id {} does not exist", channel_id),
            ),
            Self::GenericWithMessage(status, msg) => (status, msg),
//            Self::Generic(status) => (status, "".to_string()),
        };
        (status, Json(json!({ "error": err_msg.as_str() }))).into_response()
    }
}

pub type Result<T> = result::Result<T, AppError>;


impl From<anyhow::Error> for AppError {
    fn from(inner: anyhow::Error) -> Self {
        AppError::InternalServerError(inner.to_string())
    }
}