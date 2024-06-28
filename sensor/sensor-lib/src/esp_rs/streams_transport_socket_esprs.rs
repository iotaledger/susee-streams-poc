use std::{
    clone::Clone,
    fmt,
    rc::Rc,
    time::Duration,
};

use anyhow::{Result};

use async_trait::async_trait;

use hyper::{
    http::{
        StatusCode,
        Request
    },
    Body
};

use esp_idf_svc::{
    http::client::{
        Configuration as HttpConfiguration,
    },
};

use streams::{
    Address,
    transport::Transport,
    TransportMessage,
};

use lets::{
    error::{
        Error as LetsError,
        Result as LetsResult,
    },
};

use streams_tools::{
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
    http::{
        RequestBuilderStreams,
        MapLetsError,
    },
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        LinkedMessage,
        trans_msg_encode,
        trans_msg_len,
    },
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    StreamsTransport
};

use crate::{
    esp_rs::hyper_esp_rs_tools::{
        HyperEsp32Client,
        UserAgentName,
        SimpleHttpResponse,
    }
};

fn is_http_status_success(status: u16) -> bool {
    300 > status && status >= 200
}

#[derive(Clone)]
pub struct StreamsTransportSocketEspRsOptions {
    pub http_url: String,
}

impl Default for StreamsTransportSocketEspRsOptions {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL.to_string()
        }
    }
}

impl fmt::Display for StreamsTransportSocketEspRsOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpClientOptions: http_url: {}", self.http_url)
    }
}

#[derive(Clone)]
pub struct StreamsTransportSocketEspRs {
    request_builder: RequestBuilderStreams,
    esp_http_client_opt: HttpConfiguration,
    compressed: CompressedStateManager,
    initialization_cnt: u8,
}

impl StreamsTransport for StreamsTransportSocketEspRs {
    type Options = StreamsTransportSocketEspRsOptions;

    fn new(options: Option<StreamsTransportSocketEspRsOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Creating new HttpClient using options: {}", options);
        let mut esp_http_client_opt = HttpConfiguration::default();
        esp_http_client_opt.timeout = Some(Duration::from_secs(120));
        Self {
            request_builder: RequestBuilderStreams::new(options.http_url.as_str()),
            initialization_cnt: 0,
            esp_http_client_opt,
            compressed: CompressedStateManager::new(),
        }
    }

    fn set_initialization_cnt(&mut self, value: u8) {
        self.initialization_cnt = value;
    }
}

impl StreamsTransportSocketEspRs
{
    async fn request<'a>(&mut self, request: Request<Body>, http_client: &'a mut HyperEsp32Client) -> Result<SimpleHttpResponse> {
        let response = http_client.send(request).await?;

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response.status == StatusCode::ALREADY_REPORTED {
            log::debug!("[fn request()] Received StatusCode::ALREADY_REPORTED - Set use_compressed_msg = true");
            self.compressed.set_use_compressed_msg(true);
        }
        Ok(response)
    }

    async fn send_message_via_http(&mut self, msg: &LinkedMessage) -> LetsResult<()> {
        let req = if self.compressed.get_use_compressed_msg() {
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg, self.initialization_cnt);
            self.request_builder.send_compressed_message(&cmpr_message, None)
                .map_err(|e| LetsError::External(e.into()))?
        } else {
            self.request_builder.send_message(msg)
                .map_err(|e| LetsError::External(e.into()))?
        };

        let mut http_client = HyperEsp32Client::new(&self.esp_http_client_opt, UserAgentName::Main);
        self.request(req, &mut http_client).await
            .map_err(|e| LetsError::External(e.into()))?;
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &Address) -> LetsResult<LinkedMessage> {
        log::debug!("[fn recv_message_via_http()]");
        log::debug!("[fn recv_message_via_http()] EspHttpClient created");
        let req = if self.compressed.get_use_compressed_msg() {
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link, self.initialization_cnt);
            self.request_builder.receive_compressed_message_from_address(&cmpr_link, None)
                .map_err(|e| LetsError::External(e.into()))?
        } else {
            self.request_builder.receive_message_from_address(link)
                .map_err(|e| LetsError::External(e.into()))?
        };
        let mut http_client = HyperEsp32Client::new(&self.esp_http_client_opt, UserAgentName::Main);
        let response = self.request(req, &mut http_client).await
            .map_err(|e| LetsError::External(e.into()))?;
        self.handle_recv_message_response(response, link).await
    }

    async fn handle_recv_message_response<'a>(&mut self, response: SimpleHttpResponse, link: &Address) -> LetsResult<LinkedMessage> {
        log::debug!("[fn handle_recv_message_response()] check for retrials");
        // TODO: Implement following retrials using EspTimerService if needed.
        // May be StatusCode::CONTINUE is handled by the EspHttpClient
        if response.status == StatusCode::CONTINUE {
            log::warn!("[fn handle_recv_message_response()] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
            // let periodic = getPeriodicTimer(Duration::from_millis(500), move || {
            //     response = send_hyper_request_via_esp_http(
            //             self.request_builder.receive_message_from_address(link)?
            //         ).await?;
            // });

            // let mut interval = time::interval(Duration::from_millis(500));
            // while response.status() == StatusCode::CONTINUE {
            //     interval.tick().await;
            //     response = send_hyper_request_via_esp_http(
            //        self.request_builder.receive_message_from_address(link)?
            //     ).await?;
            // }
        }

        if is_http_status_success(response.status.as_u16()) {
            log::debug!("[fn handle_recv_message_response()] StatusCode is successful: {}", response.status);
            log::info!("[fn handle_recv_message_response()] create LinkedMessage ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}",
                       response.body.len(),
                       response.body.as_slice()
            );
            let body = <TransportMessage as BinaryPersist>::try_from_bytes(&response.body).unwrap();
            log::debug!("[fn handle_recv_message_response()] return ret_val");
            Ok(LinkedMessage { link: link.clone(), body })
        } else {
            log::error!("[fn handle_recv_message_response()] StatusCode is not OK");
            Err(MapLetsError::from_http_status_codes(
                response.status,
                Some(link.clone()),
                None
            ))
        }
    }
}

impl CompressedStateSend for StreamsTransportSocketEspRs {
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
impl<'a> Transport<'a> for StreamsTransportSocketEspRs
{
    type Msg = TransportMessage;
    type SendResponse = ();

    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> LetsResult<Self::SendResponse> {
        log::info!("[fn send_message()] Sending message with {} bytes tangle-message-payload:\n{}\n",
                 trans_msg_len(&msg), trans_msg_encode(&msg));
        self.send_message_via_http(&LinkedMessage {
            link: address,
            body: msg
        }).await
    }

    async fn recv_messages(&mut self, _address: Address) -> LetsResult<Vec<Self::Msg>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, address: Address) -> LetsResult<Self::Msg> {
        log::debug!("[fn recv_message()]");
        let ret_val = self.recv_message_via_http(&address).await;
        log::debug!("[fn recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[fn recv_message()] ret_val Ok");
                log::info!("[fn recv_message()] Receiving message with {} bytes tangle-message-payload:\n{}\n",
                    msg.body_len(), msg.body_hex_encode())
            },
            Err(err) => {
                log::error!("[fn recv_message()] Received streams error: '{}'", err);
                ()
            }
        }
        ret_val.map(|linked_msg| linked_msg.body)
    }
}