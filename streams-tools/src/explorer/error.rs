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


// Usually utoipa uses the IntoResponses trait to map AppError to http status codes.
// As several AppError options use the same http status code, this will not work for us.
// https://docs.rs/utoipa/latest/utoipa/trait.IntoResponses.html

#[allow(dead_code)]
#[derive(Debug, ToSchema)]
pub enum AppError {
    /// An internal server error occurred
    InternalServerError(String),
    /// At least one condition query parameter is needed
    AtLeastOneConditionNeeded(String),
    /// A channel with the specified channel-id does not exist
    ChannelDoesNotExist(String),
    /// A generic error occurred, see http status code and message for more details
    GenericWithMessage(StatusCode, String),
//     /// A generic error occurred, see http status code for more details
//    Generic(StatusCode),
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