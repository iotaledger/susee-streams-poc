use hyper::{
    Body,
    http::{
        Response,
        StatusCode,
        Error,
        request::{
            Builder,
            Request,
        }
    }
};

#[derive(Clone)]
pub struct RequestBuilderTools {
    pub uri_prefix: String,
}

impl RequestBuilderTools {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            uri_prefix: String::from(uri_prefix)
        }
    }

    pub fn get_request_builder(self: &Self) -> Builder {
        Request::builder().header("User-Agent", "streams-client/1.0")
    }

    pub fn get_uri(self: &Self, path: &str) -> String {
        format!("{}{}", self.uri_prefix, path)
    }
}

pub fn get_response_404() -> Result<Response<Body>,Error> {
    let mut not_found = Response::default();
    *not_found.status_mut() = StatusCode::NOT_FOUND;
    Ok(not_found)
}
