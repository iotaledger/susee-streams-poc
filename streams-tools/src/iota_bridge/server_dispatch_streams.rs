use std::{
    str::FromStr,
    rc::Rc,
    cell::RefCell,
    convert::TryInto,
    borrow::BorrowMut,
};

use base64::engine::{
    general_purpose::STANDARD,
    Engine,
};

use anyhow::{anyhow, bail};

use async_trait::async_trait;

use hyper::{
    Body,
    http::{
        Response,
        Result,
        StatusCode,
    }
};

use streams::{
    Address,
};

use lets::{
    transport::{
        Transport,
    },
    message::TransportMessage,
    address::{
        AppAddr,
    }
};

use crate::{
    binary_persist::{
        LinkedMessage,
        TangleMessageCompressed,
        TangleAddressCompressed,
        BinaryPersist,
        StreamsApiFunction,
        StreamsApiRequest,
        TANGLE_ADDRESS_BYTE_LEN,
        as_msg_id,
        trans_msg_encode,
        trans_msg_len,
    },
    http::{
        ScopeConsume,
        DispatchScope,
        http_tools::{
            get_response_400,
            get_response_500,
            get_dev_eui_from_str,
        },
        http_protocol_streams::{
            ServerDispatchStreams,
            URI_PREFIX_STREAMS,
        },
        iota_bridge_error::IotaBridgeError,
        get_final_http_status,
    },
    dao_helpers,
    ok_or_bail_internal_error_response_500
};

use super::{
    helpers::{
        log_anyhow_err_and_respond_500,
        log_lets_err_and_respond_mapped_status_code,
        DispatchScopeValue,
        DispatchScopeKey,
        write_to_scope,
    },
    LoraWanNodeDataStore,
    PendingRequestDataStore,
    dao::{
        LoraWanNode,
        pending_request,
        PendingRequest,
    },
    streams_transport_pool::{
        StreamsTransportPool,
        StreamsTransportPoolImpl,
        TransportHandle
    },
    streams_node_health::HealthChecker,
    error_handling_strategy::ErrorHandlingStrategy,
};

#[async_trait(?Send)]
pub trait TransportFactory: Clone {
    type Output;
    async fn new_transport<'a>(&self) -> Rc<RefCell<Self::Output>>;
}

static mut STREAMS_TRANSPORT_POOL: Option<Box<dyn StreamsTransportPool>> = None;


impl<'a> Drop for TransportHandle<'a> {
    fn drop(&mut self) {
        unsafe {
            match STREAMS_TRANSPORT_POOL.borrow_mut() {
                Some(pool) => {
                    pool.release_transport(&self);
                },
                None => {
                    log::error!("STREAMS_TRANSPORT_POOL.borrow_mut() failed")
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct DispatchStreams {
    error_handling: ErrorHandlingStrategy,
    lorawan_nodes: LoraWanNodeDataStore,
    pending_requests: PendingRequestDataStore,
    scope: Option<Rc<dyn DispatchScope>>,
    health_checker: HealthChecker,
}

impl DispatchStreams {
    pub fn new<TransportFactoryT>(
        error_handling: ErrorHandlingStrategy,
        transport_factory: TransportFactoryT,
        lorawan_nodes: LoraWanNodeDataStore,
        pending_requests: PendingRequestDataStore,
        health_checker: HealthChecker,
    ) -> Self
    where
        TransportFactoryT: TransportFactory + 'static,
        for<'a> <TransportFactoryT as TransportFactory>::Output: Transport<'a, Msg = TransportMessage, SendResponse = TransportMessage> + 'static
    {
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe shared queue instance
            //       based on Arc::new(Mutex::new(......)) as been described here
            //       https://stackoverflow.com/questions/60996488/passing-additional-state-to-rust-hyperserviceservice-fn
            if STREAMS_TRANSPORT_POOL.is_none() {
                STREAMS_TRANSPORT_POOL = Some(Box::new(StreamsTransportPoolImpl::<TransportFactoryT>::new(
                    transport_factory
                )));
            }
        }
        Self {
            error_handling,
            lorawan_nodes,
            pending_requests,
            scope: None,
            health_checker,
        }
    }

    fn write_channel_id_to_scope(&self, link: &Address) {
        if let Some(scope) = &self.scope {
            write_to_scope(scope, DispatchScopeValue::StreamsChannelId(link.base().to_string()));
        }
    }

    fn write_buffered_message_to_scope(&self, message_to_be_buffered_in_db: &LinkedMessage) {
        if let Some(scope) = &self.scope {
            write_to_scope(scope, DispatchScopeValue::AddBufferedMessageToDb((*message_to_be_buffered_in_db).clone()));
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

    fn handle_lora_wan_node_not_known(&self, dev_eui: String, address: TangleAddressCompressed, streams_api_request: StreamsApiRequest) -> Result<Response<Body>> {
        log::error!("[fn handle_lora_wan_node_not_known()] \
            The lorawan_node with dev_eui {} and initialization_cnt {} is not known by this iota-bridge instance. \
            The current request will be stored by this IOTA-Bridge for later retransmit. \
            Please provide the missing streams channel ID using the '/streams/retransmit' api function.",
                    dev_eui,
                    address.initialization_cnt,
        );
        let msgid = address.msgid.as_bytes().try_into().unwrap();
        let streams_api_request_bytes = Self::get_streams_api_request_bytes(streams_api_request);

        let new_pending_request = PendingRequest::new(
            dev_eui,
            msgid,
            address.initialization_cnt,
            streams_api_request_bytes,
        );

        let resp_body = match self.pending_requests.write_item_to_db(&new_pending_request) {
            Ok(request_key) => {
                Body::from(request_key.to_le_bytes().to_vec())
            }
            Err(err) => return get_response_500(format!("Could not write new pending_request to local database: {}", err).as_str())
        };
        Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .body(resp_body)
    }

    fn get_streams_api_request_bytes(streams_api_request: StreamsApiRequest) -> Vec<u8> {
        let request_bytes = streams_api_request.needed_size();
        let mut streams_api_request_bytes = Vec::<u8>::with_capacity(request_bytes);
        streams_api_request_bytes.resize(request_bytes, 0);
        streams_api_request.to_bytes(streams_api_request_bytes.as_mut_slice()).expect("Error on persisting streams_api_request");
        streams_api_request_bytes
    }

    fn decode_request_key_for_retransmit(request_key: String) -> anyhow::Result<i64> {
        let request_key_base64_decoded = STANDARD.decode(request_key.as_str())?;
        let request_key: <pending_request::PendingRequestDaoManager as dao_helpers::DaoManager>::PrimaryKeyType =
            i64::from_le_bytes(request_key_base64_decoded.try_into().expect("u64 slice with incorrect length"));
        Ok(request_key)
    }

    fn get_pending_request(self: &mut Self, request_key_i64: &i64) -> anyhow::Result<PendingRequest> {
        let pending_request = match self.pending_requests.get_item(&request_key_i64) {
            Ok(req_and_cb) => req_and_cb.0,
            Err(err) => {
                bail!("[fn  get_pending_request] pending_requests.get_item returned an error for request_key_i64 {}. Error: {}", request_key_i64, err);
            }
        };
        Ok(pending_request)
    }

    fn write_new_lorawan_node_to_db(self: &mut Self, pending_request: &PendingRequest, channel_id: &AppAddr, initialization_cnt: u8) -> anyhow::Result<()> {
        let lorawan_node = LoraWanNode {
            dev_eui: pending_request.dev_eui.clone(),
            initialization_cnt,
            streams_channel_id: channel_id.to_string()
        };
        self.lorawan_nodes.write_item_to_db(&lorawan_node)?;
        Ok(())
    }

    fn remove_pending_request_from_db(self: &mut Self, pending_request: &PendingRequest) -> anyhow::Result<()> {
        if let Some(req_key) = pending_request.request_key {
            self.pending_requests.delete_item_in_db(&req_key)?;
        } else {
            bail!("Provided pending_request.request_key is None")
        }
        Ok(())
    }

    fn set_request_needs_registered_lorawan_node_on_scope_to_true(self: &mut Self) {
        if let Some(scope) = self.scope.as_ref() {
            // This is normally controlled by the iota_bridge_request header flags.
            // For compressed messages this flag is set to true.
            // See Self::get_success_response_status_code() for more details.
            write_to_scope(scope, DispatchScopeValue::RequestNeedsRegisteredLorawanNode(true));
        }
    }

    async fn get_lorawan_node(self: &mut Self, dev_eui_str: &String, link: &TangleAddressCompressed) -> Option<LoraWanNode> {
        let mut ret_val: Option<LoraWanNode> = None;
        match self.lorawan_nodes.get_item(dev_eui_str) {
            Ok(node_and_cb) => {
                let node = node_and_cb.0;
                if node.initialization_cnt == link.initialization_cnt {
                    ret_val = Some(node)
                } else {
                    log::warn!("[fn get_lorawan_node()] DevEUI: {} - LoraWanNode has initialization_cnt {} but message needs initialization_cnt {}",
                               dev_eui_str, node.initialization_cnt, link.initialization_cnt);
                }
            },
            Err(err) => {
                log::warn!("[fn get_lorawan_node()] DevEUI: {} - lorawan_nodes.get_item returned an error: {}", dev_eui_str, err);
            }
        }
        ret_val
    }

    async fn send_message_when_streams_node_is_healthy(&mut self, message: &LinkedMessage) -> Result<Response<Body>> {
        unsafe {
            match STREAMS_TRANSPORT_POOL.borrow_mut() {
                Some(pool) => {
                    if let Some(mut transport) = pool.get_transport().await {
                        let res = transport.send_message(message.link, message.body.clone()).await;
                        std::mem::drop(transport);
                        match res {
                            Ok(_) => {
                                self.write_channel_id_to_scope(&message.link);
                            },
                            Err(err) => {
                                if self.error_handling == ErrorHandlingStrategy::BufferMessagesOnValidationErrors {
                                    log::error!("[fn send_message] Received error: '{}'.\nAdding buffered_message to db: {}", err, message.link);
                                    self.write_buffered_message_to_scope(message);
                                } else {
                                    log::error!("[fn send_message] Received error: '{}'.\nReturning HTTP error {} for message: {}",
                                                err, IotaBridgeError::http_error_description(IotaBridgeError::ValidationFailed), message.link);
                                    return IotaBridgeError::get_response(IotaBridgeError::ValidationFailed,
                                                                         "Validation of the correct storage of the message failed");
                                }
                            }
                        }
                        Response::builder()
                            .status(self.get_success_response_status_code())
                            .body(Default::default())
                    } else {
                        log_anyhow_err_and_respond_500(anyhow!("Could not get available streams transport client from pool"), "send_message")
                    }
                },
                None => {
                    log_anyhow_err_and_respond_500(anyhow!("Could not get transport pool"), "send_message")
                }
            }
        }
    }

    async fn receive_message_from_address_when_streams_node_is_healthy(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        let address = Address::from_str(address_str).unwrap();
        unsafe {
            match STREAMS_TRANSPORT_POOL.borrow_mut() {
                Some(pool) => {
                    if let Some(mut transport) = pool.get_transport().await {
                        let message = transport.recv_message(address).await;
                        std::mem::drop(transport);
                        match message {
                            Ok(msg) => {
                                println_receive_message_from_address_for_received_message(&msg);
                                self.write_channel_id_to_scope(&address);
                                let mut buffer: Vec<u8> = vec![0;BinaryPersist::needed_size(&msg)];
                                let size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                                log::debug!("[fn receive_message_from_address()] Returning binary data via socket connection. length: {} bytes, data:\n\
{:02X?}\n", size.unwrap_or_default(), buffer);
                                Response::builder().status(self.get_success_response_status_code())
                                    .body(buffer.into())
                            },
                            Err(err) => {
                                log::info!("Address msg_index is: {}", hex::encode(address.to_msg_index()));
                                log_lets_err_and_respond_mapped_status_code(err, "receive_message_from_address")
                            }
                        }
                    } else {
                        log_anyhow_err_and_respond_500(
                            anyhow!("Could not get available streams transport client from pool"),
                            "receive_message_from_address"
                        )
                    }
                },
                None => {
                    log_anyhow_err_and_respond_500(
                        anyhow!("Could not transport pool"),"receive_message_from_address"
                    )
                }
            }
        }
    }
}

impl DispatchStreams {
    async fn retransmit_receive_compressed_message_from_address(self: &mut Self, pending_request: PendingRequest) -> Result<Response<Body>> {
        let cmpr_addr_str =
            TangleAddressCompressed {
                msgid: as_msg_id(pending_request.msg_id.as_slice()),
                initialization_cnt: pending_request.initialization_cnt,
            }
                .to_string();

        self.set_request_needs_registered_lorawan_node_on_scope_to_true();
        self.receive_compressed_message_from_address(cmpr_addr_str.as_str(), pending_request.dev_eui.as_str()).await
    }

    async fn retransmit_send_compressed_message(self: &mut Self, pending_request: PendingRequest, mut message: TangleMessageCompressed) -> Result<Response<Body>> {
        if pending_request.request_key.is_none() {
            let err = anyhow::anyhow!("Received pending_request without request_key");
            return Ok(log_anyhow_err_and_respond_500(err, "retransmit_send_compressed_message").unwrap());
        };
        message.dev_eui = get_dev_eui_from_str(pending_request.dev_eui.as_str())?;
        self.set_request_needs_registered_lorawan_node_on_scope_to_true();
        self.send_compressed_message(&message).await
    }

    async fn check_health(&self) -> Option<Result<Response<Body>>>{
        match self.health_checker.is_healthy().await {
            Ok(healthy) => {
                if healthy {
                    log::debug!("[fn check_health] Streams Node is healthy");
                    None
                } else {
                    log::error!("[fn check_health] Streams Node is currently not healthy. Returning 503 http response.");
                    Some(IotaBridgeError::get_response(IotaBridgeError::NotHealthy,
                                                       "Streams Node is currently not healthy"))
                }
            },
            Err(e) => {
                log::error!("[fn check_health] Checking Streams Node health returned an error. Returning 503 http response. Error: {}", e);
                Some(IotaBridgeError::get_response(IotaBridgeError::NotHealthy,
                                                   format!("Checking Streams Node health returned an error: {}", e).as_str()))
            }
        }
    }
}

static LINK_LENGTH: usize = TANGLE_ADDRESS_BYTE_LEN;

fn println_send_message_for_incoming_message(message: &LinkedMessage) {
    log::info!("[fn println_send_message_for_incoming_message()] Incoming Message to attach to tangle with absolut length of {} bytes. Data:
{}", trans_msg_len(&message.body) + LINK_LENGTH, trans_msg_encode(&message.body)
    );
}

fn println_receive_message_from_address_for_received_message(message: &TransportMessage) {
    log::info!("[fn receive_message_from_address()] Received Message from tangle with absolut length of {} bytes. Data:
{}
", trans_msg_len(&message) + LINK_LENGTH, trans_msg_encode(&message)
    );
}

fn println_retransmit_for_received_message(request_key: &String, channel_id: &AppAddr, initialization_cnt: u8, streams_req: &StreamsApiRequest) {
    log::info!(
        "[fn retransmit()] Incoming request_key '{}' to retransmit cashed StreamsApiRequest for LorawanNode with channel_id {}.
Initialization Count: {}
Request key:
{}
", request_key, channel_id.to_string(), initialization_cnt, streams_req
    );
}

#[async_trait(?Send)]
impl ServerDispatchStreams for DispatchStreams {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_STREAMS }

    async fn send_message(&mut self, message: &LinkedMessage) -> Result<Response<Body>> {
        println_send_message_for_incoming_message(message);
        if let Some(err_response) = self.check_health().await {
            return err_response
        }
        self.send_message_when_streams_node_is_healthy(message).await
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        log::debug!("[fn receive_message_from_address()] Incoming request for address: {}", address_str);
        if let Some(err_response) = self.check_health().await {
            return err_response
        }
        self.receive_message_from_address_when_streams_node_is_healthy(address_str).await
    }

    async fn receive_messages_from_address(self: &mut Self, _address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn send_compressed_message(
        self: &mut Self, message: &TangleMessageCompressed) -> Result<Response<Body>>
    {
        let dev_eui = match String::from_utf8(message.dev_eui.clone()) {
            Ok(eui_str) => eui_str,
            Err(err) => return get_response_400(format!(
                "Binary data provided for dev_eui could not be converted into an utf8 string. Error: {}", err).as_str())
        };

        if let Some(lora_wan_node) = self.get_lorawan_node(&dev_eui, &message.link).await {
            let uncompressed_message = match message.to_tangle_message(lora_wan_node.streams_channel_id.as_str()) {
                Ok(msg) => msg,
                Err(err) => return get_response_500(format!("Error: {}", err).as_str())
            };
            self.send_message(&uncompressed_message).await
        }
        else {
            let streams_api_request = StreamsApiRequest{
                api_function: StreamsApiFunction::SendCompressedMessage,
                cmpr_address: "".to_string(), // address is not needed for send_message
                cmpr_message: message.clone(),
            };
            return self.handle_lora_wan_node_not_known(dev_eui, message.link.clone(), streams_api_request)
        }
    }

    async fn receive_compressed_message_from_address(self: &mut Self, cmpr_addr_str: &str, dev_eui_str: &str) -> Result<Response<Body>> {
        let cmpr_addr = TangleAddressCompressed::from_str(cmpr_addr_str)
            .expect(format!("Error on deserializing cmpr_addr from string value '{}'", cmpr_addr_str).as_str());

        if let Some(lora_wan_node) = self.get_lorawan_node(&dev_eui_str.to_string(), &cmpr_addr).await {
            let full_address_str = TangleAddressCompressed::build_tangle_address_str(
                cmpr_addr.msgid.to_string().as_str(),
                lora_wan_node.streams_channel_id.as_str()
            );
            self.receive_message_from_address(full_address_str.as_str()).await
        }
        else {
            let streams_api_request = StreamsApiRequest{
                api_function: StreamsApiFunction::ReceiveCompressedMessageFromAddress,
                cmpr_address: cmpr_addr_str.to_string(),
                cmpr_message: TangleMessageCompressed::default(), // message is not needed for receive_message
            };
            return self.handle_lora_wan_node_not_known(dev_eui_str.to_string(), cmpr_addr, streams_api_request);
        }
    }

    async fn retransmit(self: &mut Self, request_key: String, channel_id: AppAddr, initialization_cnt: u8) -> Result<Response<Body>> {
        let request_key_i64 = ok_or_bail_internal_error_response_500!(Self::decode_request_key_for_retransmit(request_key.clone()));
        let pending_request = ok_or_bail_internal_error_response_500!(self.get_pending_request(&request_key_i64));
        ok_or_bail_internal_error_response_500!(self.write_new_lorawan_node_to_db(&pending_request, &channel_id, initialization_cnt));

        let streams_req = StreamsApiRequest::try_from_bytes(pending_request.streams_api_request.as_slice())
            .expect("Error on deserializing StreamsApiRequest");

        println_retransmit_for_received_message(&request_key, &channel_id, initialization_cnt, &streams_req);

        let mut ret_val = match streams_req.api_function {
            StreamsApiFunction::SendCompressedMessage => {
                self.retransmit_send_compressed_message(pending_request.clone(), streams_req.cmpr_message).await?
            }
            StreamsApiFunction::ReceiveCompressedMessageFromAddress => {
                self.retransmit_receive_compressed_message_from_address(pending_request.clone()).await?
            }
        };

        if StatusCode::is_success(&ret_val.status()) {
            ok_or_bail_internal_error_response_500!(self.remove_pending_request_from_db(&pending_request));
            *ret_val.status_mut() = get_final_http_status(&ret_val.status(), true);
        }

        Ok(ret_val)
    }
}

#[async_trait(?Send)]
impl ScopeConsume for DispatchStreams {
    fn set_scope(&mut self, scope: Rc<dyn DispatchScope>) {
        self.scope = Some(scope);
    }
}