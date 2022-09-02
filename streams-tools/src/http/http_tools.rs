use hyper::{
    Body,
    body,
    body::Bytes,
    http::{
        Response,
        Method,
        StatusCode,
        Error,
        request::{
            Builder,
            Request,
        }
    }
};
use crate::binary_persist::{
    BinaryPersist,
    EnumeratedPersistable,
};

use url::{
    Url,
};
use std::fmt;
use std::fmt::Formatter;

#[derive(Clone)]
pub(crate) struct RequestBuilderTools {
    pub uri_prefix: String,
}

impl RequestBuilderTools {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            uri_prefix: String::from(uri_prefix)
        }
    }

    pub fn get_request_builder() -> Builder {
        Request::builder().header("User-Agent", "streams-client/1.0")
    }

    pub fn get_uri(self: &Self, path: &str) -> String {
        format!("{}{}", self.uri_prefix, path)
    }

    pub fn send_enumerated_persistable_args<T: BinaryPersist>(&self, enumerated_persistable_args: T, path: &str) -> Result<Request<Body>,Error> {
        let mut buffer: Vec<u8> = vec![0; enumerated_persistable_args.needed_size()];
        enumerated_persistable_args.to_bytes(buffer.as_mut_slice()).expect("Persisting into binary data failed");

        Self::get_request_builder()
            .method("POST")
            .uri(self.get_uri(path).as_str())
            .body(Body::from(buffer))
    }
}

pub(crate) fn get_response_404() -> Result<Response<Body>,Error> {
    get_response_with_status_code(StatusCode::NOT_FOUND)
}

pub(crate) fn get_response_500() -> Result<Response<Body>,Error> {
    get_response_with_status_code(StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn get_response_with_status_code(status_code: StatusCode) -> Result<Response<Body>,Error> {
    let builder = Response::builder()
        .status(status_code);
    builder.body(Default::default())
}


// Use the the persisted Command::XXXX_XXXX_XXXX instead as Response<Body>
pub(crate) fn get_body_bytes_from_enumerated_persistable<T: EnumeratedPersistable + BinaryPersist>(enumerated_persistable: &T) -> Result<[u8; T::LENGTH_BYTES],Error> {
    let mut buffer: [u8; T::LENGTH_BYTES] = [0; T::LENGTH_BYTES];
    enumerated_persistable.to_bytes(&mut buffer).unwrap();
    Ok(buffer)
}

pub enum DispatchedRequestStatus {
    Initial,
    DeserializedLorawanRest,
    LorawanRest404,
}

impl fmt::Display for DispatchedRequestStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DispatchedRequestStatus::Initial => write!(f, "Initial"),
            DispatchedRequestStatus::DeserializedLorawanRest => write!(f, "DeserializedLorawanRest"),
            DispatchedRequestStatus::LorawanRest404 => write!(f, "LorawanRest404"),
        }
    }
}

impl Default for DispatchedRequestStatus {
    fn default() -> Self {
        DispatchedRequestStatus::Initial
    }
}

pub struct DispatchedRequestParts {
    pub req_url: Url,
    pub method: Method,
    pub path: String,
    pub binary_body: Vec<u8>,
    pub status: DispatchedRequestStatus,
}

impl<'a> DispatchedRequestParts {
    pub async fn new(req: Request<Body>) -> anyhow::Result<DispatchedRequestParts> {
        let uri_str = req.uri().to_string();
        // unfortunately we need to specify a scheme and domain to use Url::parse() correctly
        let uri_base = Url::parse("http://this-can-be-ignored.com").unwrap();
        let req_url = uri_base.join(&uri_str).unwrap();
        let method = req.method().clone();

        // In case of a POST request move the binary body into a buffer
        let binary_body: Vec<u8>;
        let body_bytes: Bytes;
        if req.method() == Method::POST {
            body_bytes = body::to_bytes(req.into_body()).await.unwrap();
            binary_body = Vec::<u8>::from(body_bytes);
        } else {
            binary_body = Vec::<u8>::new();
        }

        let ret_val = DispatchedRequestParts {
            req_url: req_url.to_owned(),
            status: DispatchedRequestStatus::default(),
            method: method.to_owned(),
            path: String::from(req_url.path()),
            binary_body,
        };

        Ok(ret_val)
    }

    pub fn log_and_return_404(&self, fn_name_to_log: &str) -> Result<Response<Body>,Error> {
        log::debug!("[{}] could not dispatch method {} for path '{}'. Returning 404.", fn_name_to_log, self.method, self.path);
        get_response_404()
    }
}
