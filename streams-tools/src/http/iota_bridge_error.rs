use std::{
    fmt,
};

use hyper::{
    Body,
    http::{
        StatusCode,
        Response,
        Error
    }
};

use super::{
    http_tools::{
        get_response_503,
        get_response_507,
        get_response_500,
    }
};

#[derive(Eq, PartialEq)]
#[derive(Clone)]
pub enum IotaBridgeError {
    NotHealthy,
    ValidationFailed,
    Unknown,
}

impl IotaBridgeError {
    pub const NOT_HEALTHY: &'static str = "NOT-HEALTHY";
    pub const VALIDATION_FAILED: &'static str = "VALIDATION-FAILED";
    pub const UNKNOWN: &'static str = "UNKNOWN";
    
    pub fn value(&self) -> &'static str {
        match self {
            IotaBridgeError::NotHealthy => IotaBridgeError::NOT_HEALTHY,
            IotaBridgeError::ValidationFailed => IotaBridgeError::VALIDATION_FAILED,
            IotaBridgeError::Unknown => IotaBridgeError::UNKNOWN,
        }
    }

    pub fn http_error_description(self) -> &'static str {
        match self {
            IotaBridgeError::NotHealthy => "503 - Service Unavailable",
            IotaBridgeError::ValidationFailed => "507 - Insufficient Storage",
            IotaBridgeError::Unknown => "500 - Internal Server Error",
        }
    }

    pub fn get_response(self, description: &str) -> Result<Response<Body>,Error> {
        match self {
            IotaBridgeError::NotHealthy => get_response_503(description),
            IotaBridgeError::ValidationFailed => get_response_507(description),
            IotaBridgeError::Unknown => get_response_500(description),
        }
    }

    pub fn is_iota_bridge_error(http_err: StatusCode) -> bool {
        match http_err {
            StatusCode::SERVICE_UNAVAILABLE => true,
            StatusCode::INSUFFICIENT_STORAGE => true,
            StatusCode::INTERNAL_SERVER_ERROR => true,
            _ => false
        }
    }
}

impl fmt::Display for IotaBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value())
    }
}