use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};
use iota_streams::core::Errors;

use crate::http::{
    http_protocol_streams::MapStreamsErrors,
    DispatchScope,
};

use std::rc::Rc;

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

pub fn log_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
    println!("[HttpClientProxy - {}] Error: {}", fn_name, err);

    // // Following implementation does not work because currently it is not possible to access
    // // The streams error value. Instead we expect a MessageLinkNotFoundInTangle error to
    // // make the susee POC run at all.
    // // TODO: Check how to access the streams error value and fix the implementation here
    // let streams_error = &MapStreamsErrors::get_indicator_for_uninitialized();
    // for cause in err.chain() {
    //     if let Some(streams_err) = cause.downcast_ref::<Errors>() {
    //         streams_error = streams_err.clone();
    //         break;
    //     }
    // }
    // let mut status_code = MapStreamsErrors::to_http_status_codes(&streams_error);

    let status_code = MapStreamsErrors::to_http_status_codes(&Errors::MessageLinkNotFoundInTangle(String::from("")));
    let builder = Response::builder()
        .status(status_code);
    builder.body(Default::default())
}