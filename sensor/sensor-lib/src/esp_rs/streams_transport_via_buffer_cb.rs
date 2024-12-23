use std::{
    clone::Clone,
    rc::Rc
};

use anyhow::{Result};

use async_trait::async_trait;

use hyper::{
    http::{
        StatusCode,
    }
};

use streams::{
    Address,
    transport::Transport,
    TransportMessage,
};

use lets::{
    address::{
        AppAddr,
    },
    error::{
        Error as LetsError,
        Result as LetsResult,
    },
};

use streams_tools::{
    http::{
        RequestBuilderStreams,
        MapLetsError,
        http_protocol_streams::{
            EndpointUris,
            QueryParameters,
        },
    },
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        LinkedMessage,
        trans_msg_encode,
        trans_msg_len,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            IotaBridgeResponseParts,
        },
    },
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
    StreamsTransport,
};

use crate::request_via_buffer_cb::{
    RequestViaBufferCallback,
    RequestViaBufferCallbackOptions
};

#[derive(Clone)]
pub struct StreamsTransportViaBufferCallback {
    initialization_cnt: u8,
    request_via_cb: RequestViaBufferCallback,
    request_builder: RequestBuilderStreams,
    compressed: CompressedStateManager,
}

impl<'a> StreamsTransport for StreamsTransportViaBufferCallback {
    type Options = RequestViaBufferCallbackOptions;

    fn new(options: Option<RequestViaBufferCallbackOptions>) -> Self {
        Self {
            initialization_cnt: 0,
            request_via_cb: RequestViaBufferCallback::new(options),
            request_builder: RequestBuilderStreams::new(""),
            compressed: CompressedStateManager::new(),
        }
    }

    fn set_initialization_cnt(&mut self, value: u8) {
        self.initialization_cnt = value;
    }
}

impl StreamsTransportViaBufferCallback {

    async fn send_message_via_lorawan(&mut self, msg: &LinkedMessage) -> LetsResult<()> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // Please note the comments in fn recv_message_via_http() below
            // Same principles apply here
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg, self.initialization_cnt);
            self.request_builder
                .get_send_message_request_parts(
                    &cmpr_message,
                    EndpointUris::SEND_COMPRESSED_MESSAGE,
                    true,
                    None
                )
                .map_err(|e| LetsError::External(e.into()))?
        } else {
            self.request_builder
                .get_send_message_request_parts(
                    msg,
                    EndpointUris::SEND_MESSAGE,
                    false,
                    None
                )
                .map_err(|e| LetsError::External(e.into()))?
        };

        self.request(req_parts, msg.link.base()).await
            .map_err(|e| LetsError::External(e.into()))?;
        Ok(())
    }

    async fn recv_message_via_lorawan(&mut self, link: &Address) -> LetsResult<LinkedMessage> {
        log::debug!("[fn recv_message_via_lorawan()]");
        let req_parts = self.get_request_parts(link)
            .map_err(|e| LetsError::External(e.into()))?;
        let response = self.request(req_parts, link.base()).await
            .map_err(|e| LetsError::External(e.into()))?;

        log::debug!("[fn recv_message_via_lorawan()] check for retrials");
        // TODO: Implement following retrials for bad LoRaWAN connection using EspTimerService if needed.
        // May be we need to introduce StatusCode::CONTINUE in cases where LoRaWAN connection
        // is sometimes too bad and retries are a valid strategy to receive the response
        if response.status_code == StatusCode::CONTINUE {
            log::warn!("[fn recv_message_via_lorawan()] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
        }

        StreamsTransportViaBufferCallback::manage_response_status(&response, link)
    }

    fn manage_response_status(response: &IotaBridgeResponseParts, link: &Address) -> LetsResult<LinkedMessage> {
        if response.status_code.is_success() {
            log::debug!("[fn manage_response_status()] StatusCode is successful: {}", response.status_code);
            log::info!("[fn manage_response_status()] Received response with content length of {}", response.body_bytes.len());
            let body = <TransportMessage as BinaryPersist>::try_from_bytes(&response.body_bytes.as_slice()).unwrap();
            log::debug!("[fn manage_response_status()] return ret_val");
            Ok(LinkedMessage { link: link.clone(), body })
        } else {
            log::error!("[fn manage_response_status()] StatusCode is not OK");
            Err(MapLetsError::from_http_status_codes(
                response.status_code,
                Some(link.clone()),
                 None
            ))
        }
    }

    fn get_request_parts(&mut self, link: &Address) -> Result<IotaBridgeRequestParts> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // We do not set the dev_eui here because it will be communicated by the LoraWAN network
            // and therefore will not be send as lorawan payload.
            // Please note that due to this BinaryPersist implementation for TangleMessageCompressed
            // does not serialize/deserialize the dev_eui in general.
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link, self.initialization_cnt);
            self.request_builder.get_receive_message_from_address_request_parts(
                &cmpr_link,
                EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
                true,
                QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_CMPR_ADDR,
                None
            )?
        } else {
            self.request_builder.get_receive_message_from_address_request_parts(
                link,
                EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
                false,
                QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
                None
            )?
        };
        Ok(req_parts)
    }

    pub async fn request<'a>(&mut self, req_parts: IotaBridgeRequestParts, channel_id: AppAddr) -> Result<IotaBridgeResponseParts> {
        let buffer: Vec<u8> = req_parts.as_vecu8()?;
        log::debug!("[fn request()] IotaBridgeRequestParts bytes to send: Length: {}\n    {:02X?}", buffer.len(), buffer);
        let mut response_parts = self.request_via_cb.request_via_buffer_callback(buffer).await?;
        log::debug!("[fn request()] response_parts.status_code is {}", response_parts.status_code);

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response_parts.status_code == StatusCode::ALREADY_REPORTED {
            log::info!("[fn request()] Received StatusCode::ALREADY_REPORTED (208)- Set use_compressed_msg = true");
            self.compressed.set_use_compressed_msg(true);
        }
        if response_parts.status_code == StatusCode::UNPROCESSABLE_ENTITY {
            response_parts = self.handle_request_retransmit(response_parts, channel_id).await?;
        }

        log::info!("[fn request()] use_compressed_msg = '{}'", self.compressed.get_use_compressed_msg());
        Ok(response_parts)
    }

    async fn handle_request_retransmit(&mut self, mut response_parts: IotaBridgeResponseParts, channel_id: AppAddr) -> Result<IotaBridgeResponseParts> {
        let retransmit_request = self.request_builder.retransmit(
            &response_parts.body_bytes,
            channel_id,
            self.initialization_cnt,
        )?;
        log::info!("[fn handle_request_retransmit()] Received StatusCode::UNPROCESSABLE_ENTITY (422) - Processing {}",
            retransmit_request.uri());

        let retransmit_req_parts = IotaBridgeRequestParts::from_request(retransmit_request, false).await;
        let retransmit_req_bytes: Vec<u8> = retransmit_req_parts.as_vecu8()?;
        response_parts = self.request_via_cb.request_via_buffer_callback(retransmit_req_bytes).await?;

        if response_parts.status_code != StatusCode::ALREADY_REPORTED {
            log::warn!("[fn handle_request_retransmit()] Expected retransmit response with status 208-ALREADY_REPORTED. Got status {}", response_parts.status_code);
            log::warn!("[fn handle_request_retransmit()] Will set use_compressed_msg to false for security reasons");
            self.compressed.set_use_compressed_msg(false);
        }

        Ok(response_parts)
    }
}

impl CompressedStateSend for StreamsTransportViaBufferCallback {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        self.compressed.subscribe_listener(listener)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        log::debug!("[fn set_initial_use_compressed_msg_state()] use_compressed_msg is set to {}", use_compressed_msg);
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }

    fn remove_listener(&mut self, handle: usize) {
        self.compressed.remove_listener(handle);
    }
}

#[async_trait(?Send)]
impl<'a> Transport<'a> for StreamsTransportViaBufferCallback
{
    type Msg = TransportMessage;
    type SendResponse = ();

    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> LetsResult<Self::SendResponse> {
        log::info!("[fn send_message()] Sending message with {} bytes tangle-message-payload:\n{}\n",
                 trans_msg_len(&msg), trans_msg_encode(&msg));
        self.send_message_via_lorawan(&LinkedMessage {
            link: address,
            body: msg
        }).await
    }

    async fn recv_messages(&mut self, _address: Address) -> LetsResult<Vec<Self::Msg>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, address: Address) -> LetsResult<Self::Msg> {
        log::debug!("[fn recv_message()]");
        let ret_val = self.recv_message_via_lorawan(&address).await;
        log::debug!("[fn recv_message()] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[fn recv_message()] ret_val Ok");
                log::info!("[fn recv_message()] Receiving message with {} bytes tangle-message-payload:\n{}\n",
                    msg.body_len(), msg.body_hex_encode())
            },
            Err(err) => {
                log::error!("[fn recv_message()] Received streams error: '{}'", err);
                // We are handling any kind of error as TransportNotAvailable event here
                // (originally caused by a transport failure like e.g. LORAWAN_NO_CONNECTION)
                // because there is no possibility to correctly match err using iota_streams::core::Error
                // items and because of following two facts:
                // 1. The error type is ignored in the Streams implementation (api/tangle/messages.rs,
                //    fn MessagesState::next(), line 697)
                //    Cite:  // Message not found or network error. Right now we are not distinguishing
                //           // between each case, ...
                //    Which results in a MessageLinkNotFoundInStore error which results in a client
                //    resync (user_manager/subscriber_manager.rs, fn SubcriberManager::subscribe()).
                // 2. The error handling is reorganised in Streams v"0.2.0".
                //    The error handling for TransportNotAvailable will be fixed for Stardust port
                //    of the susee-streams-poc
                //
                // TODO: Fix TransportNotAvailable resp. LORAWAN_NO_CONNECTION error handling in the
                //       Stardust port of the susee-streams-poc
                ()
            }
        }
        ret_val.map(|linked_msg| linked_msg.body)
    }
}