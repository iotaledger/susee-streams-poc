use std::rc::Rc;

use hyper::{
    Body,
    http::{
        Response,
        Result,
        StatusCode,
    }
};
use lets::error::Error as LetsError;

use crate::http::{
    http_protocol_streams::MapLetsError,
    DispatchScope,
};

pub struct DispatchScopeKey {}

impl DispatchScopeKey {
    pub const STREAMS_CHANNEL_ID: &'static str = "channel-id";
    pub const LORAWAN_DEV_EUI: &'static str = "lorawan-dev-eui";
    pub const REQUEST_NEEDS_REGISTERED_LORAWAN_NODE: &'static str = "request-needs-registered-lorawan-node";
    pub const ADD_NEW_LORAWAN_NODE_TO_DB: &'static str = "add-new-lorawan-node-to-db";
}

pub enum DispatchScopeValue {
    StreamsChannelId(String),
    LorawanDevEui(String),
    RequestNeedsRegisteredLorawanNode(bool),
    AddNewLorawanNodeToDb(bool),
}

pub fn write_to_scope(scope: &Rc<dyn DispatchScope>, value: DispatchScopeValue) {
    match value {
        DispatchScopeValue::StreamsChannelId(channel_id) => {
            scope.set_string(DispatchScopeKey::STREAMS_CHANNEL_ID, channel_id.as_str());
        }
        DispatchScopeValue::LorawanDevEui(dev_eui) => {
            scope.set_string(DispatchScopeKey::LORAWAN_DEV_EUI, dev_eui.as_str());
        }
        DispatchScopeValue::RequestNeedsRegisteredLorawanNode(needs_lora_node) => {
            scope.set_bool(DispatchScopeKey::REQUEST_NEEDS_REGISTERED_LORAWAN_NODE, &needs_lora_node);
        }
        DispatchScopeValue::AddNewLorawanNodeToDb(do_add_lorawan_node) => {
            scope.set_bool(DispatchScopeKey::ADD_NEW_LORAWAN_NODE_TO_DB, &do_add_lorawan_node);
        }
    }
}

pub fn log_anyhow_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
    log::error!("[IOTA-Bridge - {}] Error: {}", fn_name, err);
    let builder = Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR);
    builder.body(Default::default())
}

pub fn log_lets_err_and_respond_mapped_status_code(lets_err: LetsError, fn_name: &str) -> Result<Response<Body>> {
    log::error!("[IOTA-Bridge - {}] Error: {}", fn_name, lets_err);
    let builder = Response::builder()
        .status(MapLetsError::to_http_status_codes(&lets_err));
    builder.body(Default::default())
}

// These tests need to be started as follows:
//      > cargo test --package streams-tools --lib iota_bridge::helpers::tests --features iota_bridge
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        iota_bridge::ServerScopeProvide,
        http::ScopeProvide,
    };

    #[test]
    fn test_write_to_scope() {
        let mut scope_provide = ServerScopeProvide::new();
        let scope = scope_provide.create_new_scope();

        write_to_scope(&scope, DispatchScopeValue::StreamsChannelId(String::from("test-channel-id")));
        write_to_scope(&scope, DispatchScopeValue::LorawanDevEui(String::from("test-dev-eui")));
        write_to_scope(&scope, DispatchScopeValue::RequestNeedsRegisteredLorawanNode(true));
        write_to_scope(&scope, DispatchScopeValue::AddNewLorawanNodeToDb(true));

        assert_eq!(scope.get_string(DispatchScopeKey::STREAMS_CHANNEL_ID).unwrap(), String::from("test-channel-id"));
        assert_eq!(scope.get_string(DispatchScopeKey::LORAWAN_DEV_EUI).unwrap(), String::from("test-dev-eui"));
        assert_eq!(scope.get_bool(DispatchScopeKey::REQUEST_NEEDS_REGISTERED_LORAWAN_NODE).unwrap(), true);
        assert_eq!(scope.get_bool(DispatchScopeKey::ADD_NEW_LORAWAN_NODE_TO_DB).unwrap(), true);
    }

    #[test]
    fn test_log_err_and_respond_500() {
        let err = anyhow::anyhow!("test error");
        let fn_name = "test_fn_name";
        let response = log_err_and_respond_500(err, fn_name).unwrap();
        // We expect a 510 because currently it is not possible to access the streams error value.
        // Instead we expect a MessageLinkNotFoundInTangle error to make the susee POC run at all.
        assert_eq!(response.status(), 510);
    }
}