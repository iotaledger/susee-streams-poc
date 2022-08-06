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
    binary_persist_command::{
        Command,
        SubscribeToAnnouncement,
        RegisterKeyloadMessage,
        StartSendingMessages,
    },
    http::http_protocol_tools::{
        RequestBuilderTools,
        get_response_404,
        get_body_bytes_from_enumerated_persistable,
    },
};

use url::{
    form_urlencoded::Parse
};

use iota_streams::core::async_trait;

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

impl EndpointUris {
    pub const FETCH_NEXT_COMMAND: &'static str  = "/command/next";
    pub const SUBSCRIBE_TO_ANNOUNCEMENT: &'static str  = "/command/subscribe_to_announcement";
    pub const REGISTER_KEYLOAD_MSG: &'static str  = "/command/register_keyload_msg";
    pub const PRINTLN_SUBSCRIBER_STATUS: &'static str  = "/command/println_subscriber_status";
    pub const CLEAR_CLIENT_STATE: &'static str  = "/command/clear_client_state";
    pub const SEND_MESSAGES: &'static str  = "/command/send_messages";
}

pub struct QueryParameters {}

impl QueryParameters {
}

#[derive(Clone)]
pub struct RequestBuilderCommand {
    tools: RequestBuilderTools,
}

impl RequestBuilderCommand {
    pub fn new(uri_prefix: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix)
        }
    }

    pub fn fetch_next_command(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::FETCH_NEXT_COMMAND).as_str())
            .body(Body::empty())
    }

    pub fn println_subscriber_status(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::PRINTLN_SUBSCRIBER_STATUS).as_str())
            .body(Body::empty())
    }

    pub fn clear_client_state(self: &Self) -> Result<Request<Body>> {
        self.tools.get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::CLEAR_CLIENT_STATE).as_str())
            .body(Body::empty())
    }

    pub fn send_message(self: &Self, message_template_key: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            StartSendingMessages{
                wait_seconds_between_repeats: 30,
                message_template_key: message_template_key.to_string()
            },
            EndpointUris::SEND_MESSAGES
        )
    }

    pub fn subscribe_to_announcement(self: &Self, announcement_link_str: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            SubscribeToAnnouncement{ announcement_link: announcement_link_str.to_string() },
            EndpointUris::SUBSCRIBE_TO_ANNOUNCEMENT
        )
    }

    pub fn register_keyload_msg(self: &Self, keyload_msg_link_str: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            RegisterKeyloadMessage{ keyload_msg_link: keyload_msg_link_str.to_string() },
            EndpointUris::REGISTER_KEYLOAD_MSG
        )
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchCommand {
    async fn fetch_next_command(self: &mut Self) -> Result<Response<Body>>;
    async fn register_remote_command(self: &mut Self, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_command(method: &Method, path: &str, body_bytes: &[u8], _query_pairs: &Parse<'_>, callbacks: &mut impl ServerDispatchCommand) -> Result<Response<Body>> {
    match (method, path) {
        (&Method::GET, EndpointUris::FETCH_NEXT_COMMAND) => {
            callbacks.fetch_next_command().await
        },

        (&Method::GET, EndpointUris::PRINTLN_SUBSCRIBER_STATUS) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Command::PRINTLN_SUBSCRIBER_STATUS)?;
            callbacks.register_remote_command(&buffer, "println_subscriber_status").await
        },

        (&Method::GET, EndpointUris::CLEAR_CLIENT_STATE) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Command::CLEAR_CLIENT_STATE)?;
            callbacks.register_remote_command(&buffer, "clear_client_state").await
        },

        (&Method::POST, EndpointUris::SEND_MESSAGES) => {
            callbacks.register_remote_command(body_bytes, "send_message").await
        },

        (&Method::POST, EndpointUris::SUBSCRIBE_TO_ANNOUNCEMENT) => {
            callbacks.register_remote_command(body_bytes, "subscribe_to_announcement").await
        },

        (&Method::POST, EndpointUris::REGISTER_KEYLOAD_MSG) => {
            callbacks.register_remote_command(body_bytes, "register_keyload_msg").await
        },

        // Return the 404 Not Found for other routes.
        _ => get_response_404()
    }
}