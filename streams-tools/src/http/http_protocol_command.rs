#![allow(non_snake_case)]

use std::cell::{
    Cell,
    RefCell
};

use async_trait::async_trait;

use anyhow::{
    Result as AnyResult,
    bail
};

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
    binary_persist::{
        Command,
        SubscribeToAnnouncement,
        RegisterKeyloadMessage,
        StartSendingMessages,
        DevEuiHandshakeCmd,
        EnumeratedPersistable,
        EnumeratedPersistableArgs,
    },
    STREAMS_TOOLS_CONST_ANY_DEV_EUI
};

use super::{
    http_dispatch_scope::ScopeConsume,
    http_tools::{
        RequestBuilderTools,
        DispatchedRequestParts,
        get_body_bytes_from_enumerated_persistable,
        get_response_400,
        PathSegments
    }
};

// TODO s:
// * Create a enum based Uri and parameter management for API endpoints similar to
//   https://github.com/hyperium/http/blob/master/src/method.rs
//
// * Create an errors enum similat to
//   iota-streams-core/src/errors/error_messages.rs

pub struct EndpointUris {}

pub const URI_PREFIX_COMMAND: &'static str = "/command";

impl EndpointUris {
    pub const FETCH_NEXT_COMMAND: &'static str  = "/command/next";
    pub const SUBSCRIBE_TO_ANNOUNCEMENT: &'static str  = "/command/subscribe_to_announcement";
    pub const REGISTER_KEYLOAD_MSG: &'static str  = "/command/register_keyload_msg";
    pub const PRINTLN_SUBSCRIBER_STATUS: &'static str  = "/command/println_subscriber_status";
    pub const CLEAR_CLIENT_STATE: &'static str  = "/command/clear_client_state";
    pub const SEND_MESSAGES: &'static str  = "/command/send_messages";
    pub const DEV_EUI_HANDSHAKE: &'static str  = "/command/dev_eui_handshake";
    // TODO: Currently there is no endpoint for the STOP_FETCHING_COMMANDS command because it is used
    //       only internally in the X86/PC sensor application. In case the mangement-console should be able
    //       to exit the fetch_message loop on the remote controlled ESP32 Sensor the REST function for
    //       STOP_FETCHING_COMMANDS needs to be implemented.

    pub fn get_uri___fetch_next_command(dev_eui: &str) -> String {
        format!("{}/{}", Self::FETCH_NEXT_COMMAND, dev_eui)
    }
    pub fn get_uri___subscribe_to_announcement(dev_eui: &str) -> String {
        format!("{}/{}", Self::SUBSCRIBE_TO_ANNOUNCEMENT, dev_eui)
    }
    pub fn get_uri___register_keyload_msg(dev_eui: &str) -> String {
        format!("{}/{}", Self::REGISTER_KEYLOAD_MSG, dev_eui)
    }
    pub fn get_uri___println_subscriber_status(dev_eui: &str) -> String {
        format!("{}/{}", Self::PRINTLN_SUBSCRIBER_STATUS, dev_eui)
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
pub struct RequestBuilderCommand {
    tools: RequestBuilderTools,
    dev_eui: RefCell<String>,
    // Allows clients of this struct (usually CommandFetchers) to manage if they should do
    // a DevEUI-Handshake first.
    // If dev_eui_handshake_first is true a CommandFetcher should do a DevEUI-Handshake
    // with the management-console before any other commands are fetched:
    // If true:
    //      RequestBuilderCommand::fetch_next_command() will use the dev_eui 'ANY'
    //      (STREAMS_TOOLS_CONST_ANY_DEV_EUI) to fetch a DevEuiHandshake
    //      command.
    //      The CommandFetcher shall not accept any other type of command and shall fetch
    //      commands until a DevEuiHandshake command has been received.
    //      After a DevEuiHandshake command has been received,
    //      RequestBuilderCommand::fetch_next_command() will use the dev_eui defined by the
    //      'dev_eui' field of this struct.
    // If false (default):
    //      RequestBuilderCommand::fetch_next_command() will use the dev_eui defined by the
    //      'dev_eui' field of this struct.
    pub dev_eui_handshake_first: Cell<bool>,
}

impl RequestBuilderCommand {
    pub fn new(uri_prefix: &str, dev_eui: &str, dev_eui_handshake_first: bool) -> Self {
        Self {
            tools: RequestBuilderTools::new(uri_prefix),
            dev_eui: RefCell::new(dev_eui.to_string()),
            dev_eui_handshake_first: Cell::new(dev_eui_handshake_first),
        }
    }

    pub fn set_dev_eui(&self, dev_eui: &str) {
        *self.dev_eui.borrow_mut() = dev_eui.to_string();
    }

    pub fn get_dev_eui(&self) -> String {
        self.dev_eui.borrow().clone()
    }

    pub fn get_dev_eui_for_fetch_next_command_request(&self) -> String {
        if self.dev_eui_handshake_first.get() {
            STREAMS_TOOLS_CONST_ANY_DEV_EUI.to_string()
        } else {
            self.dev_eui.borrow().clone()
        }
    }

    pub fn manage_dev_eui_by_received_command(&self, cmd: &Command) -> AnyResult<()> {
        if cmd.as_u8() != Command::NO_COMMAND.as_u8() &&
            self.dev_eui_handshake_first.get()
        {
            if cmd == DevEuiHandshakeCmd::INSTANCE {
                log::info!("[fn manage_dev_eui_by_received_command()] DevEUI: {} - Received DevEuiHandshake command.",
                    self.get_dev_eui_for_fetch_next_command_request()
                );
                self.dev_eui_handshake_first.set(false);
                log::info!("[fn manage_dev_eui_by_received_command()] dev_eui for next command fetches will be '{}'",
                    self.get_dev_eui_for_fetch_next_command_request()
                );
            } else {
                bail!("[fn manage_dev_eui_by_received_command()] DevEUI: {} - Received command {} instead of DevEuiHandshake",
                    self.get_dev_eui_for_fetch_next_command_request(),
                    cmd,
                );
            }
        }
        Ok(())
    }

    pub fn fetch_next_command(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___fetch_next_command(
                    self.get_dev_eui_for_fetch_next_command_request().as_str())
            ).as_str())
            .body(Body::empty())
    }

    pub fn println_subscriber_status(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___println_subscriber_status(
                    self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }

    pub fn clear_client_state(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___clear_client_state(
                    self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }

    pub fn send_message_in_endless_loop(self: &Self, message_template_key: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            StartSendingMessages{
                wait_seconds_between_repeats: 30,
                message_template_key: message_template_key.to_string()
            },
            EndpointUris::get_uri___send_messages(
                self.dev_eui.borrow().as_str()
            ).as_str()
        )
    }

    pub fn subscribe_to_announcement(self: &Self, announcement_link_str: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            SubscribeToAnnouncement{ announcement_link: announcement_link_str.to_string() },
            EndpointUris::get_uri___subscribe_to_announcement(
                self.dev_eui.borrow().as_str()
            ).as_str()
        )
    }

    pub fn register_keyload_msg(self: &Self, keyload_msg_link_str: &str) -> Result<Request<Body>> {
        self.tools.send_enumerated_persistable_args(
            RegisterKeyloadMessage{ keyload_msg_link: keyload_msg_link_str.to_string() },
            EndpointUris::get_uri___register_keyload_msg(
                self.dev_eui.borrow().as_str()
            ).as_str()
        )
    }

    pub fn dev_eui_handshake(self: &Self) -> Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(
                &EndpointUris::get_uri___dev_eui_handshake(
                    self.dev_eui.borrow().as_str())
            ).as_str())
            .body(Body::empty())
    }
}

#[async_trait(?Send)]
pub trait ServerDispatchCommand: ScopeConsume {
    fn get_uri_prefix(&self) -> &'static str;
    async fn fetch_next_command(self: &mut Self, dev_eui: &str ) -> Result<Response<Body>>;
    async fn register_remote_command(self: &mut Self, dev_eui: &str, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>>;
}

pub async fn dispatch_request_command(req_parts: &DispatchedRequestParts, callbacks: &mut impl ServerDispatchCommand) -> Result<Response<Body>> {
    let segments = match PathSegments::new_from_path(req_parts.path.as_str()) {
        Ok(seg) => seg,
        Err(err) => return get_response_400(format!(
            "The dev_eui could not be parsed out of the specified url. Error: {}", err).as_str())
    };
    match (&req_parts.method, segments.main.as_str()) {
        (&Method::GET, EndpointUris::FETCH_NEXT_COMMAND) => {
            callbacks.fetch_next_command(segments.last.as_str()).await
        },

        (&Method::GET, EndpointUris::PRINTLN_SUBSCRIBER_STATUS) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Command::PRINTLN_SUBSCRIBER_STATUS)?;
            callbacks.register_remote_command(segments.last.as_str(), &buffer, "println_subscriber_status").await
        },

        (&Method::GET, EndpointUris::CLEAR_CLIENT_STATE) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Command::CLEAR_CLIENT_STATE)?;
            callbacks.register_remote_command(segments.last.as_str(), &buffer, "clear_client_state").await
        },

        (&Method::POST, EndpointUris::SEND_MESSAGES) => {
            callbacks.register_remote_command(segments.last.as_str(), &req_parts.binary_body, "send_message").await
        },

        (&Method::POST, EndpointUris::SUBSCRIBE_TO_ANNOUNCEMENT) => {
            callbacks.register_remote_command(segments.last.as_str(), &req_parts.binary_body, "subscribe_to_announcement").await
        },

        (&Method::POST, EndpointUris::REGISTER_KEYLOAD_MSG) => {
            callbacks.register_remote_command(segments.last.as_str(), &req_parts.binary_body, "register_keyload_msg").await
        },

        (&Method::GET, EndpointUris::DEV_EUI_HANDSHAKE) => {
            let buffer = get_body_bytes_from_enumerated_persistable(&Command::DEV_EUI_HANDSHAKE)?;
            callbacks.register_remote_command(segments.last.as_str(), &buffer, "dev_eui_handshake").await
        },

        // Return the 404 Not Found for other routes.
        _ => req_parts.log_and_return_404("dispatch_request_command", "")
    }
}