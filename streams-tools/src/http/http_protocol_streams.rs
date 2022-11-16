use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        Method,
        StatusCode,
    }
};

use iota_streams::{
    app::
    {
        transport::tangle::{
            TangleMessage,
            TangleAddress
        },
    },
    app_channels::api::DefaultF,
    core::{
        async_trait,
        Errors,
    },
};

use crate::{
    binary_persist::{
        BinaryPersist,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            HttpMethod,
        }
    },
};

use super::http_tools::{
    RequestBuilderTools,
};
use crate::http::http_tools::DispatchedRequestParts;

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

pub const URI_PREFIX_STREAMS: &'static str = "/message";

impl EndpointUris {
    pub const SEND_MESSAGE: &'static str = "/message/send";
    pub const RECEIVE_MESSAGE_FROM_ADDRESS: &'static str  = "/message";
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
pub struct RequestBuilderStreams {
    tools: RequestBuilderTools,
}

impl RequestBuilderStreams {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn get_send_message_request_parts(self: &Self, message: &TangleMessage) -> Result<IotaBridgeRequestParts> {
        let mut buffer: Vec<u8> = vec![0; message.needed_size()];
        message.to_bytes(buffer.as_mut_slice()).expect("Persisting into binary data failed");

        Ok(IotaBridgeRequestParts::new(
            HttpMethod::POST,
            self.tools.get_uri(EndpointUris::SEND_MESSAGE),
            buffer
        ))
    }

    pub fn send_message(self: &Self, message: &TangleMessage) -> Result<Request<Body>> {
        self.get_send_message_request_parts(message)?
            .into_request(RequestBuilderTools::get_request_builder())
    }

    pub fn get_receive_message_from_address_request_parts(self: &Self, address: &TangleAddress) -> Result<IotaBridgeRequestParts> {
        let uri = format!("{}?{}={}",
                          self.tools.get_uri(EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS).as_str(),
                          QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
                          address.to_string()
        );

        Ok(IotaBridgeRequestParts::new(
            HttpMethod::GET,
            uri,
            Vec::<u8>::new()
        ))
    }

    pub fn receive_message_from_address(self: &Self, address: &TangleAddress) -> Result<Request<Body>> {
        self.get_receive_message_from_address_request_parts(address)?
            .into_request(RequestBuilderTools::get_request_builder())
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchStreams {
    fn get_uri_prefix(&self) -> &'static str;
    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage) -> Result<Response<Body>>;
    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
    async fn receive_messages_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_streams(req_parts: &DispatchedRequestParts, callbacks: &mut impl ServerDispatchStreams ) -> Result<Response<Body>> {
    match (&req_parts.method, req_parts.path.as_str()) {

        (&Method::POST, EndpointUris::SEND_MESSAGE) => {
            let tangle_msg: TangleMessage = TangleMessage::try_from_bytes(&req_parts.binary_body).unwrap();
            callbacks.send_message::<DefaultF>(&tangle_msg).await
        },


        (&Method::GET, EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS) => {
            let address_key_val: Vec<_> = req_parts.req_url.query_pairs().collect();
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

        // Return the 404 Not Found for other routes.
        _ => req_parts.log_and_return_404("dispatch_request_streams")
    }
}