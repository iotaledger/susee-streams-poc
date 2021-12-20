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

use iota_streams::{
    app::
    {
        transport::tangle::{
            TangleMessage,
            TangleAddress,
            client::iota_client::Url,
        },
    },
    app_channels::api::DefaultF,
    core::{
        async_trait,
        Errors,
    },
};

use std::ops::Deref;

use crate::BinaryPersist;

pub enum ClientCommand {
    SubscribeToAnnouncement,
    RegisterKeyloadMessage,
}


// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

impl EndpointUris {
    pub const SEND_MESSAGE: &'static str = "/message/send";
    pub const RECEIVE_MESSAGE_FROM_ADDRESS: &'static str  = "/message";
    pub const FETCH_NEW_COMMANDS: &'static str  = "message/fetch_new";
}

pub struct QueryParameters {}

impl QueryParameters {
    pub const RECEIVE_MESSAGE_FROM_ADDRESS: &'static str  = "addr";
}

// Maps Streams Errors to http status codes
//
// TODO:
// Currently only two streams errors are mapped to http status codes which will lead to errors
// in future development. There are possibilities to persist errors in a more generic way e.g.
// https://docs.rs/strum_macros/0.23.1/strum_macros/index.html
//
// Ideally all streams errors can be returned including their values using http headers
// in a generic implementation.

trait MapStreamsErrorsAssociations {
    const NOT_PROVIDED: &'static str;
}

pub struct MapStreamsErrors {}

impl MapStreamsErrorsAssociations for MapStreamsErrors{
    const NOT_PROVIDED: &'static str = "Not provided";
}

impl MapStreamsErrors {
    pub fn to_http_status_codes(streams_error: &Errors) -> StatusCode {
        match streams_error {
            Errors::MessageLinkNotFoundInTangle(_) => StatusCode::NOT_EXTENDED,
            Errors::MessageNotUnique(_) => StatusCode::VARIANT_ALSO_NEGOTIATES,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn from_http_status_codes(http_error: StatusCode, comment: Option<String>) -> Errors {
        let comment = comment.unwrap_or(String::from(MapStreamsErrors::NOT_PROVIDED));
        match http_error {
            StatusCode::NOT_EXTENDED => Errors::MessageLinkNotFoundInTangle(comment),
            StatusCode::VARIANT_ALSO_NEGOTIATES => Errors::MessageNotUnique(comment),
            _ => MapStreamsErrors::get_indicator_for_uninitialized(),
        }
    }

    // Can be used to initialize a Errors variable and make sure that this specific value is not
    // mapped by to_http_status_codes() to a specific http status code
    pub fn get_indicator_for_uninitialized() -> Errors {
        Errors::LengthMismatch(0, 0)
    }
}

#[derive(Clone)]
pub struct RequestBuilder {
    uri_prefix: String,
}

impl RequestBuilder {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            uri_prefix: String::from(uri_prefix)
        }
    }

    fn get_request_builder(self: &Self) -> Builder {
        Request::builder().header("User-Agent", "streams-client/1.0")
    }

    fn get_uri(self: &Self, path: &str) -> String {
        format!("{}{}", self.uri_prefix, path)
    }

    pub fn send_message<F>(self: &Self, message: &TangleMessage<F>) -> Result<Request<Body>> {
        let mut buffer: Vec<u8> = vec![0; message.needed_size()];
        message.to_bytes(buffer.as_mut_slice()).expect("Persisting into binary data failed");

        self.get_request_builder()
            .method("POST")
            .uri(self.get_uri(EndpointUris::SEND_MESSAGE).as_str())
            .body(Body::from(buffer))
    }

    pub fn receive_message_from_address(self: &Self, address: &TangleAddress) -> Result<Request<Body>> {
        let uri = format!("{}?{}={}",
              self.get_uri(EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS).as_str(),
              QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
              address.to_string()
        );
        self.get_request_builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
    }

    pub fn fetch_new_commands(self: &Self) -> (Vec<ClientCommand>, Result<Request<Body>>) {
        let client_commands: Vec<ClientCommand> = Vec::new();
        let req_result = self.get_request_builder()
            .method("GET")
            .uri(self.get_uri(EndpointUris::FETCH_NEW_COMMANDS).as_str())
            .body(Body::empty());

        // TODO: Parse client commands out of req_result body and return the resulting list of
        // commands and arguments. May be then the Request<Body> is no more needed and the
        // return value can be of type Result<Vec<(ClientCommand, Vec<ClientCommandArgs>))>>
        (client_commands, req_result)
    }
}

#[async_trait(?Send)]
pub trait ServerDispatch {
    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage<F>) -> Result<Response<Body>>;
    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
    async fn receive_messages_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
    async fn fetch_new_commands(self: &mut Self) -> Result<Response<Body>>;
}

pub async fn dispatch_request(req: Request<Body>, callbacks: &mut impl ServerDispatch) -> Result<Response<Body>> {

    let uri_str = req.uri().to_string();
    // unfortunately we need to specify a scheme and domain to use Url::parse() correctly
    let uri_base = Url::parse("http://this-can-be-ignored.com").unwrap();
    let req_url = uri_base.join(&uri_str).unwrap();
    let query_pairs = req_url.query_pairs();
    let path = req_url.path();
    match (req.method(), path) {

        (&Method::POST, EndpointUris::SEND_MESSAGE) => {
            let bytes = body::to_bytes(req.into_body()).await.unwrap();
            let tangle_msg: TangleMessage<DefaultF> = TangleMessage::try_from_bytes(bytes.deref()).unwrap();
            callbacks.send_message(&tangle_msg).await
        },


        (&Method::GET, EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS) => {
            let address_key_val: Vec<_> = query_pairs.collect();
            if address_key_val.len() != 1 {
                panic!("[http_protocoll - RECEIVE_MESSAGE_FROM_ADDRESS] Wrong number of query parameters.\
                Specify the message address using /{}?{}={}",
                       EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
                       QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
                        "<MESSAGE-ADDRESS-GOES-HERE>")
            }
            if address_key_val[0].0 != QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS {
                panic!("[http_protocoll - RECEIVE_MESSAGE_FROM_ADDRESS] Query parameter {} is not specified",
                    QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS)
            }
            callbacks.receive_message_from_address(&*address_key_val[0].1).await
        },

        (&Method::GET, EndpointUris::FETCH_NEW_COMMANDS) => {
            callbacks.fetch_new_commands().await
        },

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}