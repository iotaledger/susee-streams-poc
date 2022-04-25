use hyper::{
    Body,
    body,
    http::{
        Request,
        Response,
        Result,
        Method,
        StatusCode,
        request::Builder,
    }
};

use crate::client::http::{
    http_protocol_streams::{
        ServerDispatchStreams,
        dispatch_request_streams,
    },
    http_protocol_command::{
        ServerDispatchCommand,
        dispatch_request_command,
    },
};

use url::{
    Url,
    form_urlencoded::Parse
};

use std::{
    ops::Deref,
};
use hyper::body::Bytes;

pub async fn dispatch_request(
    req: Request<Body>,
    streamsCallbacks: &mut impl ServerDispatchStreams,
    commandCallbacks: &mut impl ServerDispatchCommand,
) -> Result<Response<Body>> {

    let uri_str = req.uri().to_string();
    // unfortunately we need to specify a scheme and domain to use Url::parse() correctly
    let uri_base = Url::parse("http://this-can-be-ignored.com").unwrap();
    let req_url = uri_base.join(&uri_str).unwrap();
    let query_pairs = req_url.query_pairs();
    let path = req_url.path();

    let method = req.method().clone();

    // In case of a POST request move the binary body into a buffer
    let mut binary_body: &[u8];
    let body_bytes: Bytes;
    if req.method() == Method::POST {
        body_bytes = body::to_bytes(req.into_body()).await.unwrap();
        binary_body = body_bytes.deref();
    } else {
        binary_body = &[];
    }

    let mut response = dispatch_request_streams(&method, path, binary_body, &query_pairs, streamsCallbacks).await?;

    if response.status() == StatusCode::NOT_FOUND {
        response = dispatch_request_command(&method, path, binary_body, &query_pairs, commandCallbacks).await?;
    }

    Ok(response)
}