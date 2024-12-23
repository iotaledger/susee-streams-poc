use std::{
    clone::Clone,
    fmt,
    rc::Rc,
    time::{
        Duration,
    },
};

use anyhow::{
    anyhow,
    Result,
    bail
};

use async_trait::async_trait;

use tokio::time;

use hyper::{
    Client as HyperClient,
    body as hyper_body,
    Body,
    client::HttpConnector,
    http::{
        StatusCode,
        Response,
    },
    body,
};

use streams::{
    Address,
    transport::Transport,
};

use lets::{
    address::{
        AppAddr,
    },
    error::{
        Error as LetsError,
        Result as LetsResult,
    },
    message::TransportMessage,
};

use crate::{
    http::{
        RequestBuilderStreams,
        MapLetsError,
        http_tools::RequestBuilderTools,
        http_protocol_lorawan_rest::RequestBuilderLorawanRest,
        http_protocol_streams::{
            EndpointUris,
            QueryParameters
        }
    },
    streams_transport::STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            IotaBridgeResponseParts
        },
        LinkedMessage,
        trans_msg_encode,
        trans_msg_len,
    },
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
    StreamsTransport
};

#[derive(Clone)]
pub struct StreamsTransportSocketOptions {
    pub http_url: String,
    pub failover_secondary_http_url: Option<String>,
    pub dev_eui: Option<String>,
    pub use_lorawan_rest: bool,
}

impl StreamsTransportSocketOptions {
    pub fn new(http_url: String) -> Self {
        let mut ret_val = Self::default();
        ret_val.http_url = http_url;
        ret_val
    }
}

impl Default for StreamsTransportSocketOptions {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL.to_string(),
            failover_secondary_http_url: None,
            dev_eui: None,
            use_lorawan_rest: false,
        }
    }
}

impl fmt::Display for StreamsTransportSocketOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StreamsTransportSocketOptions:
              http_url:          {},
              failover_http_url: {:?},
              dev_eui:           {},
              use_lorawan_rest:  {}",
            self.http_url,
            self.failover_secondary_http_url,
            if let Some(eui) = &self.dev_eui {eui.as_str()} else {""},
            self.use_lorawan_rest
        )
    }
}

#[derive(Clone)]
pub struct StreamsTransportSocket {
    hyper_client: HyperClient<HttpConnector, Body>,
    request_builder_streams: RequestBuilderStreams,
    request_builder_lorawan_rest: RequestBuilderLorawanRest,
    compressed: CompressedStateManager,
    initialization_cnt: u8,
    opt: StreamsTransportSocketOptions,
}

impl StreamsTransport for StreamsTransportSocket {
    type Options = StreamsTransportSocketOptions;

    fn new(options: Option<StreamsTransportSocketOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::info!("[fn new()] Initializing instance with options:\n{}\n", options);
        Self {
            hyper_client: HyperClient::new(),
            request_builder_streams: RequestBuilderStreams::new(options.http_url.as_str()),
            request_builder_lorawan_rest: RequestBuilderLorawanRest::new(options.http_url.as_str()),
            compressed: CompressedStateManager::new(),
            initialization_cnt: 0,
            opt: options,
        }
    }

    fn set_initialization_cnt(&mut self, value: u8) {
        self.initialization_cnt = value;
    }
}

impl StreamsTransportSocket
{
    fn get_lorawan_rest_request_parts(&self, req_parts: IotaBridgeRequestParts) -> Result<IotaBridgeRequestParts>{
        let mut buffer: Vec<u8> = vec![0; req_parts.needed_size()];
        req_parts.to_bytes(buffer.as_mut_slice())?;
        if let Some(dev_eui) = self.opt.dev_eui.as_ref() {
            let lorawan_rest_req_parts = self.request_builder_lorawan_rest
                .get_post_binary_request_parts(buffer, dev_eui.as_str())?;
            Ok(lorawan_rest_req_parts)
        } else {
            bail!("You need to specify a dev_eui in the StreamsTransportSocketOptions to use the lorawan-rest API with this StreamsTransportSocket" )
        }
    }

    async fn request(&mut self, req_parts: IotaBridgeRequestParts, channel_id: AppAddr) -> Result<Response<Body>> {
        let mut response = self.get_request_response(req_parts).await?;

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response.status() == StatusCode::ALREADY_REPORTED {
            self.compressed.set_use_compressed_msg(true);
        }
        if response.status() == StatusCode::UNPROCESSABLE_ENTITY {
            response = self.handle_request_retransmit(response, channel_id).await?;
        }
        Ok(response)
    }

    async fn get_request_response(&mut self, req_parts: IotaBridgeRequestParts) -> Result<Response<Body>> {
        let req_parts_primary_iota_bridge= if self.opt.use_lorawan_rest {
            self.get_lorawan_rest_request_parts(req_parts)?
        } else {
                req_parts.clone()
        };

        let mut do_try_failover_iota_bridge = true;
        let request_primary_iota_bridge = req_parts_primary_iota_bridge.clone()
            .into_request(RequestBuilderTools::get_request_builder())?;
        log::debug!("[fn get_request_response] Sending request to primary iota-bridge: URI: {}", request_primary_iota_bridge.uri());
        let mut response = match self.hyper_client.request(request_primary_iota_bridge).await {
            Ok(resp) => resp,
            Err(err) => {
                log::debug!("[fn get_request_response] Accessing iota-bridge responded in error: {}", err);
                if let Some(failover_http_url) = &self.opt.failover_secondary_http_url {
                    do_try_failover_iota_bridge = false;
                    self.send_request_to_secondary_iota_bridge(
                        req_parts_primary_iota_bridge.clone(),
                        failover_http_url,
                        format!("Accessing iota-bridge responded in error: {}", err),
                    ).await?
                } else {
                    return Err(anyhow!(err))
                }
            }
        };

        if do_try_failover_iota_bridge && !response.status().is_success() {
            log::debug!("[fn get_request_response] Primary iota-bridge responded with status {}", response.status());
            if let Some(failover_http_url) = &self.opt.failover_secondary_http_url {
                response = self.send_request_to_secondary_iota_bridge(
                    req_parts_primary_iota_bridge,
                    failover_http_url,
                    format!("Primary iota-bridge responded with http error status {}", response.status()),
                ).await?;
            }
        }

        if self.opt.use_lorawan_rest {
            response = StreamsTransportSocket::handle_lorawan_rest_response(response).await?;
        }
        Ok(response)
    }

    async fn send_request_to_secondary_iota_bridge(
        &self,
        mut cloned_req_parts: IotaBridgeRequestParts,
        failover_http_url: &String,
        err_info: String,
    )  -> Result<Response<Body>> {
        cloned_req_parts.uri = cloned_req_parts.uri.replace(self.opt.http_url.as_str(), failover_http_url.as_str());
        log::warn!("[fn send_request_to_secondary_iota_bridge] {}. Sending request to secondary iota-bridge. URI: {}", err_info, cloned_req_parts.uri);
        let failover_req = cloned_req_parts.into_request(RequestBuilderTools::get_request_builder())?;
        self.hyper_client.request(failover_req).await.map_err(|err| anyhow!(err))
    }

    async fn handle_request_retransmit(&mut self, mut response: Response<Body>, channel_id: AppAddr) -> Result<Response<Body>> {
        let request_key_bytes = body::to_bytes(response.body_mut()).await.expect("Failed to read body bytes for retrieving the request_key");

        let retransmit_request_parts = self.request_builder_streams.get_retransmit_request_parts(
            &Vec::<u8>::from(request_key_bytes),
            channel_id,
            self.initialization_cnt,
        )?;

        let response = self.get_request_response(retransmit_request_parts).await?;

        if response.status() != StatusCode::ALREADY_REPORTED {
            log::warn!("[fn handle_request_retransmit] Expected retransmit response with status 208-ALREADY_REPORTED. Got status {}", response.status());
            log::warn!("[fn handle_request_retransmit] Will set use_compressed_msg to false for security reasons");
            self.compressed.set_use_compressed_msg(false);
        }

        Ok(response)
    }

    async fn handle_lorawan_rest_response(response: Response<Body>) -> Result<Response<Body>>{
        let mut ret_val = response;
        if ret_val.status().is_success() {
            let bytes = hyper_body::to_bytes(ret_val.into_body()).await?;
            if bytes.len() > 0 {
                match IotaBridgeResponseParts::try_from_bytes(bytes.to_vec().as_slice()) {
                    Ok(response_parts) => {
                        log::debug!("[fn handle_lorawan_rest_response()] Successfully deserialized response_parts:\n{}", response_parts);
                        if !response_parts.status_code.is_success() {
                            let err_msg = String::from_utf8(response_parts.body_bytes.clone())
                                .unwrap_or(String::from("Could not deserialize Error message from response Body"));
                            log::debug!("[fn handle_lorawan_rest_response()] Response status is not successful: Error message is:\n{}", err_msg);
                        }
                        ret_val = Response::builder()
                            .status(response_parts.status_code)
                            .body(Body::from(response_parts.body_bytes))?;
                    },
                    Err(e) => {
                        log::debug!("[fn handle_lorawan_rest_response()] Error on deserializing response_parts: {}", e);
                        bail!("Could not deserialize response binary to valid IotaBridgeResponseParts: {}", e)
                    }
                }
            } else {
                bail!("Received 0 bytes response from server.")
            }
        } else {
            log::error!("Iota-Bridge responded with http error status on lorawan-rest request. Status: {}", ret_val.status());
            bail!("Iota-Bridge responded with http error status on lorawan-rest request. Status: {}", ret_val.status())
        }
        Ok(ret_val)
    }

    async fn send_message_via_http(&mut self, msg: &LinkedMessage) -> LetsResult<()> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // In contrast to StreamsTransportViaBufferCallback we set the dev_eui here because it could be
            // used in cases where the lorawan-rest API is not used for compressed messages.
            // StreamsTransportViaBufferCallback never sets the DevEUI because it is communicated by the
            // LoraWAN network automatically (compare comment in function StreamsTransportViaBufferCallback::recv_message_via_http()
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg, self.initialization_cnt);
            self.request_builder_streams
                .get_send_message_request_parts(&cmpr_message, EndpointUris::SEND_COMPRESSED_MESSAGE, true, self.opt.dev_eui.clone())
                .map_err(|e| LetsError::External(e.into()))?
        } else {
            self.request_builder_streams
                .get_send_message_request_parts(msg, EndpointUris::SEND_MESSAGE, false, None)
                .map_err(|e| LetsError::External(e.into()))?
        };
        let channel_id = msg.link.base().clone();
        self.request(req_parts, channel_id).await.map_err(|e| LetsError::External(e))?;
        Ok(())
    }

    fn get_recv_message_request(&self, link: &Address) -> Result<IotaBridgeRequestParts> {
        let ret_val = if self.compressed.get_use_compressed_msg() {
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link, self.initialization_cnt);
            self.request_builder_streams.get_receive_message_from_address_request_parts(
                &cmpr_link,
                EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
                true,
                QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_CMPR_ADDR,
                self.opt.dev_eui.clone(),
            )
        } else {
            self.request_builder_streams.get_receive_message_from_address_request_parts(
                link,
                EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
                false,
                QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
                None,
            )
        };
        Ok(ret_val.expect("Error on creating IotaBridgeRequestParts"))
    }

    async fn recv_message_via_http(&mut self, link: &Address) -> LetsResult<LinkedMessage> {
        let req = self.get_recv_message_request(link).map_err(|e| LetsError::External(e))?;
        let channel_id = link.base().clone();
        let mut response = self.request(req, channel_id).await.map_err(|e| LetsError::External(e))?;
        // TODO: This retrials are most probably not needed because they might be handled by hyper
        //       => Clarify and remove unneeded code
        if response.status() == StatusCode::CONTINUE {
            let mut interval = time::interval(Duration::from_millis(500));
            while response.status() == StatusCode::CONTINUE {
                interval.tick().await;
                response = self.request(
                    self.get_recv_message_request(link).map_err(|e| LetsError::External(e))?,
                    link.base()
                ).await.map_err(|e| LetsError::External(e))?;
            }
        }

        if response.status().is_success() {
            let bytes = hyper_body::to_bytes(response.into_body()).await
                .map_err(|_| LetsError::External(anyhow!("Error on reading hyper_body")))?;
            let body = <TransportMessage as BinaryPersist>::try_from_bytes(&bytes)
                .map_err(|e| LetsError::External(e))?;
            Ok(LinkedMessage { link: link.clone(), body })
        } else {
            Err(MapLetsError::from_http_status_codes(
                response.status(),
                Some(link.clone()),
                Some("receive message via http".to_string())
            ))
        }
    }
}

impl CompressedStateSend for StreamsTransportSocket {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        self.compressed.subscribe_listener(listener)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }

    fn remove_listener(&mut self, handle: usize) {
        self.compressed.remove_listener(handle);
    }
}

#[async_trait(?Send)]
impl<'a> Transport<'a> for StreamsTransportSocket
{
    type Msg = TransportMessage;
    type SendResponse = ();

    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> LetsResult<Self::SendResponse> {
        log::info!("[fn send_message()] Sending message with {} bytes tangle-message-payload:\n{}\n",
                 trans_msg_len(&msg), trans_msg_encode(&msg));
        self.send_message_via_http(&LinkedMessage{
            link: address,
            body: msg
        }).await
    }

    async fn recv_messages(&mut self, _address: Address) -> LetsResult<Vec<Self::Msg>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, address: Address) -> LetsResult<Self::Msg> {
        let ret_val = self.recv_message_via_http(&address).await;
        match ret_val.as_ref() {
            Ok(msg) => log::info!("[fn recv_message()] Receiving message with {} bytes tangle-message-payload:\n{}\n",
                                msg.body_len(), msg.body_hex_encode()),
            _ => ()
        }
        ret_val.map(|linked_msg| linked_msg.body)
    }
}