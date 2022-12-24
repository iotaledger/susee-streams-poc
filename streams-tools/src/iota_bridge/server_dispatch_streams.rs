use iota_streams::{
    app::{
        transport::{
            Transport,
            tangle::{
                TangleAddress,
                TangleMessage,
                client::{
                    Client,
                }
            },
        },
    },
    core::{
        async_trait,
    }
};


use std::{
    clone::Clone,
    str::FromStr,
    rc::Rc,
};

use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    binary_persist::{
        TangleMessageCompressed,
        TangleAddressCompressed,
        BinaryPersist,
        TANGLE_ADDRESS_BYTE_LEN,
    },
    http::{
        ScopeConsume,
        DispatchScope,
        http_tools::{
            get_response_400,
            get_response_500
        },
        http_protocol_streams::{
            ServerDispatchStreams,
            URI_PREFIX_STREAMS,
        },
        get_final_http_status,
    },
};

use super::{
    helpers::{
        log_err_and_respond_500,
        DispatchScopeValue,
        DispatchScopeKey,
        write_to_scope,
    },
    LoraWanNodeDataStore,
};

use log;
use hyper::http::StatusCode;

#[derive(Clone)]
pub struct DispatchStreams {
    client: Client,
    lorawan_nodes: LoraWanNodeDataStore,
    scope: Option<Rc<dyn DispatchScope>>,
}

impl DispatchStreams
{
    pub fn new(client: &Client, lorawan_nodes: LoraWanNodeDataStore) -> Self {
        Self {
            client: client.clone(),
            lorawan_nodes,
            scope: None,
        }
    }

    fn write_channel_id_to_scope(&self, link: &TangleAddress) {
        if let Some(scope) = &self.scope {
            write_to_scope(scope, DispatchScopeValue::StreamsChannelId(link.appinst.to_string()));
        }
    }

    fn get_success_response_status_code(self: &Self) -> StatusCode {
        let mut ret_val = StatusCode::OK;
        // The iota-bridge needs to store a lorawan_node in the database if all needed data are available
        // and the lorawan_node has not already been stored in the database.
        // This requires that the ServerDispatchStreams function has been called via a
        // DispatchLorawanRest::post_binary_request() call because otherwise in case of
        // send_message() or receive_message_from_address() calls the dev_eui would not be known.
        //
        // Calls to the functions send_compressed_message() and receive_compressed_message_from_address()
        // would provide the dev_eui but these functions shall only be used by sensors in case the
        // lorawan_node already has been stored in the iota-bridge database.
        //
        // As The DispatchScopeKey::REQUEST_NEEDS_REGISTERED_LORAWAN_NODE is only added by
        // DispatchLorawanRest::post_binary_request() our first condition is that this key must exist on the scope.
        if let Some(scope) = self.scope.as_ref() {
            if scope.contains_key(DispatchScopeKey::REQUEST_NEEDS_REGISTERED_LORAWAN_NODE) {
                let request_needs_registered_lorawan_node = scope.get_bool(DispatchScopeKey::REQUEST_NEEDS_REGISTERED_LORAWAN_NODE)
                    .expect("Error on getting NEEDS_REGISTERED_LORAWAN_NODE from scope");
                // If the request needs a registered lorawan_node we don't want to add a new lorawan_node
                // to the database because this flag is only set when the Sensor already expects the iota-bridge
                // to know the lorawan_node resp. the lorawan_node is already contained in the iota-bridge database.
                let add_new_lorawan_node_to_db = !request_needs_registered_lorawan_node;
                write_to_scope(scope, DispatchScopeValue::AddNewLorawanNodeToDb(add_new_lorawan_node_to_db));
                ret_val = get_final_http_status(&StatusCode::OK, add_new_lorawan_node_to_db);
            }
        }

        ret_val
    }
}

static LINK_AND_PREVLINK_LENGTH: usize = 2 * TANGLE_ADDRESS_BYTE_LEN;

fn println_send_message_for_incoming_message(message: &TangleMessage) {
    println!(
        "-----------------------------------------------------------------\n\
[HttpClientProxy - DispatchStreams] send_message() - Incoming Message to attach to tangle with absolut length of {} bytes. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}

fn println_receive_message_from_address_for_received_message(message: &TangleMessage) {
    println!(
        "-----------------------------------------------------------------\n\
[HttpClientProxy - DispatchStreams] receive_message_from_address() - Received Message from tangle with absolut length of {} bytes. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}

#[async_trait(?Send)]
impl ServerDispatchStreams for DispatchStreams {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_STREAMS }

    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage) -> Result<Response<Body>>
    {
        println_send_message_for_incoming_message(message);
        let res = self.client.send_message(message).await;
        match res {
            Ok(_) => {
                self.write_channel_id_to_scope(&message.link);
                Response::builder().status(self.get_success_response_status_code())
                    .body(Default::default())
            },
            Err(err) => log_err_and_respond_500(err, "send_message")
        }
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        log::debug!("[HttpClientProxy - DispatchStreams] receive_message_from_address() - Incoming request for address: {}", address_str);
        let address = TangleAddress::from_str(address_str).unwrap();
        let message = Transport::<TangleAddress, TangleMessage>::
            recv_message(&mut self.client, &address).await;
        match message {
            Ok(msg) => {
                println_receive_message_from_address_for_received_message(&msg);
                self.write_channel_id_to_scope(&address);
                let mut buffer: Vec<u8> = vec![0;BinaryPersist::needed_size(&msg)];
                let size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                log::debug!("[HttpClientProxy - DispatchStreams] receive_message_from_address() - Returning binary data via socket connection. length: {} bytes, data:\n\
{:02X?}\n", size.unwrap_or_default(), buffer);
                Response::builder().status(self.get_success_response_status_code())
                    .body(buffer.into())
            },
            Err(err) => log_err_and_respond_500(err, "[HttpClientProxy - DispatchStreams] receive_message_from_address()")
        }
    }

    async fn receive_messages_from_address(self: &mut Self, _address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn send_compressed_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessageCompressed) -> Result<Response<Body>>
    {
        let dev_eui: u64 = match <u64 as BinaryPersist>::try_from_bytes(message.dev_eui.as_slice()) {
            Ok(eui_num) => eui_num,
            Err(err) => return get_response_400(format!(
                "Binary data provided for dev_eui could not be converted into an u64 number. Error: {}", err).as_str())
        };

        let dev_eui_str = dev_eui.to_string();
        let lora_wan_node = match self.lorawan_nodes.get_item(dev_eui_str.as_str()) {
            Ok(node_and_cb) => node_and_cb.0,
            Err(err) => return get_response_400(format!(
                "The provided dev_eui {} is not known. Please use REST function 'send_message' instead. Error: {}", dev_eui_str, err).as_str())
        };

        let uncompressed_message = match message.to_tangle_message(lora_wan_node.streams_channel_id.as_str()) {
            Ok(msg) => msg,
            Err(err) => return get_response_500(format!("Error: {}", err).as_str())
        };

        self.send_message::<F>(&uncompressed_message).await
    }

    async fn receive_compressed_message_from_address(self: &mut Self, msgid: &str, dev_eui_str: &str) -> Result<Response<Body>> {
        let lora_wan_node = match self.lorawan_nodes.get_item(dev_eui_str) {
            Ok(node_and_cb) => node_and_cb.0,
            Err(err) => return get_response_400(format!(
                "The provided dev_eui {} is not known. Please use REST function 'receive_message_from_address' instead. Error: {}", dev_eui_str, err).as_str())
        };

        let full_address_str = TangleAddressCompressed::build_tangle_address_str(msgid, lora_wan_node.streams_channel_id.as_str());
        self.receive_message_from_address(full_address_str.as_str()).await
    }
}

#[async_trait(?Send)]
impl ScopeConsume for DispatchStreams {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}