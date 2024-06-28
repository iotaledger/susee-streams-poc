#![allow(non_snake_case)]

use std::cell::RefCell;

use async_trait::async_trait;

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
    http::{
        ScopeConsume,
        http_tools::{
            DispatchedRequestParts,
            RequestBuilderTools,
            PathSegments,
            get_body_bytes_from_enumerated_persistable,
            get_response_400,
        }
    },
    binary_persist::{
        SubscriberStatus,
        SendMessages,
        DevEuiHandshake,
        binary_persist_confirmation::{
            Subscription,
            Confirmation
        }
    }
};

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
    pub const DEV_EUI_HANDSHAKE: &'static str  = "/confirm/dev_eui_handshake";

    pub fn get_uri___fetch_next_confirmation(dev_eui: &str) -> String {
        format!("{}/{}", Self::FETCH_NEXT_CONFIRMATION, dev_eui)
    }

    pub fn get_uri___subscription(dev_eui: &str) -> String {
        format!("{}/{}", Self::SUBSCRIPTION, dev_eui)
    }

    pub fn get_uri___subscriber_status(dev_eui: &str) -> String {
        format!("{}/{}", Self::SUBSCRIBER_STATUS, dev_eui)
    }

    pub fn get_uri___keyload_registration(dev_eui: &str) -> String {
        format!("{}/{}", Self::KEYLOAD_REGISTRATION, dev_eui)
    }

    pub fn get_uri___clear_client_state(dev_eui: &str) -> String {
        format!("{}/{}", Self::CLEAR_CLIENT_STATE, dev_eui)
    }

    pub fn get_uri___send_messages(dev_eui: &str) -> String {
        format!("{}/{}", Self::SEND_MESSAGES, dev_eui)
    }

    pub fn get_uri___dev_eui_handshake(dev_eui: &str) -> String {
        format!("{}/{}", Self::DEV_EUI_HANDSHAKE, dev_eui)
    }
}

pub struct QueryParameters {}

impl QueryParameters {
}

#[derive(Clone)]
pub struct RequestBuilderConfirm {
    tools: RequestBuilderTools,
    dev_eui: RefCell<String>,
}

impl RequestBuilderConfirm {
    pub fn new(uri_prefix: &str, dev_eui: &str) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix),
            dev_eui: RefCell::new(dev_eui.to_string()),
        }
    }

    pub fn set_dev_eui(&self, dev_eui: &str) {
        *self.dev_eui.borrow_mut() = dev_eui.to_string();
    }

    pub fn get_dev_eui(&self) -> String {
        self.dev_eui.borrow().clone()
    }

    pub fn fetch_next_confirmation(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___fetch_next_confirmation(
                    self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }

    pub fn subscription(self: &Self, subscription_link: String, pup_key: String, initialization_cnt: u8) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            Subscription{
                subscription_link,
                pup_key,
                initialization_cnt,
            },
            &EndpointUris::get_uri___subscription(self.dev_eui.borrow().as_str())
        )
    }

    pub fn subscriber_status(self: &Self, previous_message_link: String, subscription: Subscription) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            SubscriberStatus{
                previous_message_link,
                subscription,
            },
            &EndpointUris::get_uri___subscriber_status(self.dev_eui.borrow().as_str())
        )
    }

    pub fn send_messages_in_endless_loop(self: &Self) -> Result<Request<Body>> {
        // As the triggered functions will run in an endless loop
        // this confirmation will not really be sent and currently just exists to satisfy the compiler.
        self.tools.send_enumerated_persistable_args(
            SendMessages{ previous_message_link: "No previous message link available".to_string() },
            &EndpointUris::get_uri___send_messages(self.dev_eui.borrow().as_str())
        )
    }

    pub fn keyload_registration(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___keyload_registration(self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }

    pub fn clear_client_state(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___clear_client_state(self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }

    /// @param dev_eui: The real dev_eui of the sensor which will be forwarded to the
    ///                 management-console.
    ///                 Please note that there are two dev_eui values:
    ///                 (1) this fn argument, which is used for the confirmation that is
    ///                     send in the body of the request.
    ///                 (2) the dev_eui field of this RequestBuilderConfirm instance,
    ///                     which is used as url-parameter for the iota-bridge request
    ///                     so it usually should be set to 'ANY'.
    pub fn dev_eui_handshake(self: &Self, dev_eui: String) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            DevEuiHandshake{dev_eui},
            &EndpointUris::get_uri___dev_eui_handshake(self.dev_eui.borrow().as_str())
        )
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchConfirm: ScopeConsume {
    fn get_uri_prefix(&self) -> &'static str;
    async fn fetch_next_confirmation(self: &mut Self, dev_eui: &str) -> Result<Response<Body>>;
    async fn register_confirmation(self: &mut Self, dev_eui: &str, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_confirm(req_parts: &DispatchedRequestParts, callbacks: &mut impl ServerDispatchConfirm) -> Result<Response<Body>> {
    let segments = match PathSegments::new_from_path(req_parts.path.as_str()) {
        Ok(seg) => seg,
        Err(err) => return get_response_400(format!(
            "The dev_eui could not be parsed out of the specified url. Error: {}", err).as_str())
    };
    match (&req_parts.method, segments.main.as_str()) {
        (&Method::GET, EndpointUris::FETCH_NEXT_CONFIRMATION) => {
            callbacks.fetch_next_confirmation(segments.last.as_str()).await
        },

        (&Method::POST, EndpointUris::SUBSCRIPTION) => {
            callbacks.register_confirmation(segments.last.as_str(), &req_parts.binary_body, "subscription").await
        },

        (&Method::POST, EndpointUris::SUBSCRIBER_STATUS) => {
            callbacks.register_confirmation(segments.last.as_str(), &req_parts.binary_body, "subscriber_status").await
        },

        (&Method::POST, EndpointUris::SEND_MESSAGES) => {
            callbacks.register_confirmation(segments.last.as_str(), &req_parts.binary_body, "send_message").await
        },

        (&Method::GET, EndpointUris::KEYLOAD_REGISTRATION) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Confirmation::KEYLOAD_REGISTRATION)?;
            callbacks.register_confirmation(segments.last.as_str(), &buffer, "keyload_registration").await
        },

        (&Method::GET, EndpointUris::CLEAR_CLIENT_STATE) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Confirmation::CLEAR_CLIENT_STATE)?;
            callbacks.register_confirmation(segments.last.as_str(), &buffer, "clear_client_state").await
        },

        (&Method::POST, EndpointUris::DEV_EUI_HANDSHAKE) => {
            callbacks.register_confirmation(segments.last.as_str(), &req_parts.binary_body, "dev_eui_handshake").await
        },

        // Return the 404 Not Found for other routes.
        _ => req_parts.log_and_return_404("dispatch_request_confirm", "")
    }
}