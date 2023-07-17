use iota_streams::{
    app::{
        transport::{
            Transport,
            TransportDetails,
            TransportOptions,
            tangle::{
                TangleAddress,
                TangleMessage,
                AppInst
            },
        },
    },
    core::{
        async_trait,
        Result,
        err,
    },
};

use std::{
    clone::Clone,
    rc::Rc
};

use streams_tools::{http::{
    RequestBuilderStreams,
    http_protocol_streams::{
            MapStreamsErrors,
            EndpointUris,
            QueryParameters,
        },
    }, binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            IotaBridgeResponseParts,
        },
    }, compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
    StreamsTransport,
};

use hyper::{
    http::{
        StatusCode,
    }
};

use iota_client_types::{
    Details,
    SendOptions
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
    tangle_client_options: SendOptions,
    compressed: CompressedStateManager,
}

impl<'a> StreamsTransport for StreamsTransportViaBufferCallback {
    type Options = RequestViaBufferCallbackOptions;

    fn new(options: Option<RequestViaBufferCallbackOptions>) -> Self {
        Self {
            initialization_cnt: 0,
            request_via_cb: RequestViaBufferCallback::new(options),
            request_builder: RequestBuilderStreams::new(""),
            tangle_client_options: SendOptions::default(),
            compressed: CompressedStateManager::new(),
        }
    }

    fn set_initialization_cnt(&mut self, value: u8) {
        self.initialization_cnt = value;
    }
}

impl StreamsTransportViaBufferCallback {

    async fn send_message_via_lorawan(&mut self, msg: &TangleMessage) -> Result<()> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // Please note the comments in fn recv_message_via_http() below
            // Same principles apply here
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg, self.initialization_cnt);
            self.request_builder.get_send_message_request_parts(&cmpr_message, EndpointUris::SEND_COMPRESSED_MESSAGE, true, None)?
        } else {
            self.request_builder.get_send_message_request_parts(msg, EndpointUris::SEND_MESSAGE, false, None)?
        };

        self.request(req_parts, msg.link.appinst).await?;
        Ok(())
    }

    async fn recv_message_via_lorawan(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[StreamsTransportViaBufferCallback.recv_message_via_http]");
        let req_parts = self.get_request_parts(link)?;
        let response = self.request(req_parts, link.appinst).await?;

        log::debug!("[StreamsTransportViaBufferCallback.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials for bad LoRaWAN connection using EspTimerService if needed.
        // May be we need to introduce StatusCode::CONTINUE in cases where LoRaWAN connection
        // is sometimes too bad and retries are a valid strategy to receive the response
        if response.status_code == StatusCode::CONTINUE {
            log::warn!("[StreamsTransportViaBufferCallback.recv_message_via_http] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
        }

        StreamsTransportViaBufferCallback::manage_response_status(&response, link)
    }

    fn manage_response_status(response: &IotaBridgeResponseParts, link: &TangleAddress) -> Result<TangleMessage> {
        if response.status_code.is_success() {
            log::debug!("[StreamsTransportViaBufferCallback.recv_message_via_http] StatusCode is successful: {}", response.status_code);
            log::info!("[StreamsTransportViaBufferCallback.recv_message_via_http] Received response with content length of {}", response.body_bytes.len());
            let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&response.body_bytes.as_slice()).unwrap();
            log::debug!("[StreamsTransportViaBufferCallback.recv_message_via_http] return ret_val");
            Ok(ret_val)
        } else {
            log::error!("[StreamsTransportViaBufferCallback.recv_message_via_http] StatusCode is not OK");
            err!(MapStreamsErrors::from_http_status_codes(
                response.status_code,
                 Some(link.to_string())
            ))
        }
    }

    fn get_request_parts(&mut self, link: &TangleAddress) -> Result<IotaBridgeRequestParts> {
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

    pub async fn request<'a>(&mut self, req_parts: IotaBridgeRequestParts, channel_id: AppInst) -> Result<IotaBridgeResponseParts> {
        let buffer: Vec<u8> = req_parts.as_vecu8()?;
        log::debug!("[StreamsTransportViaBufferCallback.request] IotaBridgeRequestParts bytes to send: Length: {}\n    {:02X?}", buffer.len(), buffer);
        let mut response_parts = self.request_via_cb.request_via_buffer_callback(buffer).await?;
        log::debug!("[StreamsTransportViaBufferCallback::request()] response_parts.status_code is {}", response_parts.status_code);

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response_parts.status_code == StatusCode::ALREADY_REPORTED {
            log::info!("[StreamsTransportViaBufferCallback::request()] Received StatusCode::ALREADY_REPORTED (208)- Set use_compressed_msg = true");
            self.compressed.set_use_compressed_msg(true);
        }
        if response_parts.status_code == StatusCode::UNPROCESSABLE_ENTITY {
            response_parts = self.handle_request_retransmit(response_parts, channel_id).await?;
        }

        log::info!("[StreamsTransportViaBufferCallback::request()] use_compressed_msg = '{}'", self.compressed.get_use_compressed_msg());
        Ok(response_parts)
    }

    async fn handle_request_retransmit(&mut self, mut response_parts: IotaBridgeResponseParts, channel_id: AppInst) -> Result<IotaBridgeResponseParts> {
        let retransmit_request = self.request_builder.retransmit(
            &response_parts.body_bytes,
            channel_id,
            self.initialization_cnt,
        )?;
        log::info!("[StreamsTransportViaBufferCallback::handle_request_retransmit()] Received StatusCode::UNPROCESSABLE_ENTITY (422) - Processing {}",
            retransmit_request.uri());

        let retransmit_req_parts = IotaBridgeRequestParts::from_request(retransmit_request, false).await;
        let retransmit_req_bytes: Vec<u8> = retransmit_req_parts.as_vecu8()?;
        response_parts = self.request_via_cb.request_via_buffer_callback(retransmit_req_bytes).await?;

        if response_parts.status_code != StatusCode::ALREADY_REPORTED {
            log::warn!("[StreamsTransportViaBufferCallback.handle_request_retransmit] Expected retransmit response with status 208-ALREADY_REPORTED. Got status {}", response_parts.status_code);
            log::warn!("[StreamsTransportViaBufferCallback.handle_request_retransmit] Will set use_compressed_msg to false for security reasons");
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
        log::debug!("[StreamsTransportViaBufferCallback::set_initial_use_compressed_msg_state()] use_compressed_msg is set to {}", use_compressed_msg);
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }

    fn remove_listener(&mut self, handle: usize) {
        self.compressed.remove_listener(handle);
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for StreamsTransportViaBufferCallback
{
    async fn send_message(&mut self, msg: &TangleMessage) -> anyhow::Result<()> {
        log::info!("[StreamsTransportViaBufferCallback.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_lorawan(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> anyhow::Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> anyhow::Result<TangleMessage> {
        log::debug!("[StreamsTransportViaBufferCallback.recv_message]");
        let ret_val = self.recv_message_via_lorawan(link).await;
        log::debug!("[StreamsTransportViaBufferCallback.recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[StreamsTransportViaBufferCallback.recv_message] ret_val Ok");
                log::info!("[StreamsTransportViaBufferCallback.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string())
            },
            Err(err) => {
                log::error!("[StreamsTransportViaBufferCallback.recv_message] Received streams error: '{}'", err);
                ()
            }
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for StreamsTransportViaBufferCallback {
    type Details = Details;
    async fn get_link_details(&mut self, _link: &TangleAddress) -> anyhow::Result<Self::Details> {
        unimplemented!()
    }
}

impl TransportOptions for StreamsTransportViaBufferCallback {
    type SendOptions = SendOptions;
    fn get_send_options(&self) -> SendOptions {
        self.tangle_client_options.clone()
    }
    fn set_send_options(&mut self, opt: SendOptions) {
        self.tangle_client_options  = opt.clone()
    }

    type RecvOptions = ();
    fn get_recv_options(&self) {}
    fn set_recv_options(&mut self, _opt: ()) {}
}