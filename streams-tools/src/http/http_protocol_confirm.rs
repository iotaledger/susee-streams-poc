use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
        Method,
    }
};

use crate::{
    http::http_tools::{
        RequestBuilderTools,
        get_response_404,
        get_body_bytes_from_enumerated_persistable,
    },
    binary_persist::binary_persist_confirmation::{
        Subscription,
        Confirmation
    }
};

use url::{
    form_urlencoded::Parse
};

use iota_streams::core::async_trait;
use crate::binary_persist::{SubscriberStatus, SendMessages};

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

pub const URI_PREFIX_CONFIRM: &'static str = "/confirm";

impl EndpointUris {
    pub const FETCH_NEXT_CONFIRMATION: &'static str  = "/confirm/next";
    pub const SUBSCRIPTION: &'static str  = "/confirm/subscription";
    pub const SUBSCRIBER_STATUS: &'static str  = "/confirm/subscriber_status";
    pub const KEYLOAD_REGISTRATION: &'static str  = "/confirm/keyload_registration";
    pub const CLEAR_CLIENT_STATE: &'static str  = "/confirm/clear_client_state";
    pub const SEND_MESSAGES: &'static str  = "/confirm/send_messages";
}

pub struct QueryParameters {}

impl QueryParameters {
}

#[derive(Clone)]
pub struct RequestBuilderConfirm {
    tools: RequestBuilderTools,
}

impl RequestBuilderConfirm {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn fetch_next_confirmation(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::FETCH_NEXT_CONFIRMATION).as_str())
            .body(Body::empty())
    }

    pub fn subscription(self: &Self, subscription_link: String, pup_key: String) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            Subscription{
                subscription_link,
                pup_key,
            },
            EndpointUris::SUBSCRIPTION
        )
    }

    pub fn subscriber_status(self: &Self, previous_message_link: String, subscription: Subscription) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            SubscriberStatus{
                previous_message_link,
                subscription,
            },
            EndpointUris::SUBSCRIBER_STATUS
        )
    }

    pub fn send_message(self: &Self, previous_message_link: String) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            SendMessages{previous_message_link},
            EndpointUris::SEND_MESSAGES
        )
    }

    pub fn keyload_registration(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::KEYLOAD_REGISTRATION).as_str())
            .body(Body::empty())
    }

    pub fn clear_client_state(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::CLEAR_CLIENT_STATE).as_str())
            .body(Body::empty())
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchConfirm {
    fn get_uri_prefix(&self) -> &'static str;
    async fn fetch_next_confirmation(self: &mut Self) -> Result<Response<Body>>;
    async fn register_confirmation(self: &mut Self, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_confirm(method: &Method, path: &str, body_bytes: &[u8], _query_pairs: &Parse<'_>, callbacks: &mut impl ServerDispatchConfirm) -> Result<Response<Body>> {

    match (method, path) {
        (&Method::GET, EndpointUris::FETCH_NEXT_CONFIRMATION) => {
            callbacks.fetch_next_confirmation().await
        },

        (&Method::POST, EndpointUris::SUBSCRIPTION) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Confirmation::SUBSCRIPTION)?;
            callbacks.register_confirmation(&buffer, "subscription").await
        },

        (&Method::POST, EndpointUris::SUBSCRIBER_STATUS) => {
            callbacks.register_confirmation(body_bytes, "subscriber_status").await
        },

        (&Method::POST, EndpointUris::SEND_MESSAGES) => {
            callbacks.register_confirmation(body_bytes, "send_message").await
        },

        (&Method::GET, EndpointUris::KEYLOAD_REGISTRATION) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Confirmation::KEYLOAD_REGISTRATION)?;
            callbacks.register_confirmation(&buffer, "keyload_registration").await
        },

        (&Method::GET, EndpointUris::CLEAR_CLIENT_STATE) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Confirmation::CLEAR_CLIENT_STATE)?;
            callbacks.register_confirmation(&buffer, "clear_client_state").await
        },

        // Return the 404 Not Found for other routes.
        _ => {
            log::debug!("[dispatch_request_confirm] could not dispatch method {} for path '{}'. Returning 404.", method, path);
            get_response_404()
        }
    }
}