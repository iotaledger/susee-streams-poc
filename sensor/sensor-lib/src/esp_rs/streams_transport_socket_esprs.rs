use iota_streams::{
    app::{
        transport::{
            Transport,
            TransportDetails,
            TransportOptions,
            tangle::{
                TangleAddress,
                TangleMessage
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
    fmt,
    rc::Rc,
    time::Duration,
};

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

use streams_tools::{
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
    http::{
        RequestBuilderStreams,
        MapStreamsErrors,
    },
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed
    },
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL
};

use iota_client_types::{
    Details,
    SendOptions
};

use crate::{
    esp_rs::hyper_esp_rs_tools::{
        HyperEsp32Client,
        UserAgentName,
    }
};
use crate::esp_rs::hyper_esp_rs_tools::SimpleHttpResponse;

fn is_http_status_success(status: u16) -> bool {
    300 > status && status >= 200
}

pub struct StreamsTransportSocketEspRsOptions<'a> {
    pub(crate) http_url: &'a str,
}

impl Default for StreamsTransportSocketEspRsOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL
        }
    }
}

impl fmt::Display for StreamsTransportSocketEspRsOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpClientOptions: http_url: {}", self.http_url)
    }
}

#[derive(Clone)]
pub struct StreamsTransportSocketEspRs {
    request_builder: RequestBuilderStreams,
    tangle_client_options: SendOptions,
    esp_http_client_opt: HttpConfiguration,
    compressed: CompressedStateManager,
}

impl StreamsTransportSocketEspRs
{
    pub fn new(options: Option<StreamsTransportSocketEspRsOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[StreamsTransportSocketEspRs::new()] Creating new HttpClient using options: {}", options);
        let mut esp_http_client_opt = HttpConfiguration::default();
        esp_http_client_opt.timeout = Some(Duration::from_secs(60));
        Self {
            request_builder: RequestBuilderStreams::new(options.http_url),
            tangle_client_options: SendOptions::default(),
            esp_http_client_opt,
            compressed: CompressedStateManager::new(),
        }
    }

    async fn request<'a>(&mut self, request: Request<Body>, http_client: &'a mut HyperEsp32Client) -> Result<SimpleHttpResponse> {
        let response = http_client.send(request).await?;

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response.status == StatusCode::ALREADY_REPORTED {
            log::debug!("[StreamsTransportSocketEspRs::request()] Received StatusCode::ALREADY_REPORTED - Set use_compressed_msg = true");
            self.compressed.set_use_compressed_msg(true);
        }
        Ok(response)
    }

    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {
        let req = if self.compressed.get_use_compressed_msg() {
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg);
            self.request_builder.send_compressed_message(&cmpr_message, None)?
        } else {
            self.request_builder.send_message(msg)?
        };

        let mut http_client = HyperEsp32Client::new(&self.esp_http_client_opt, UserAgentName::Main);
        self.request(req, &mut http_client).await?;
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[StreamsTransportSocketEspRs.recv_message_via_http]");
        log::debug!("[StreamsTransportSocketEspRs.recv_message_via_http] EspHttpClient created");
        let req = if self.compressed.get_use_compressed_msg() {
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link);
            self.request_builder.receive_compressed_message_from_address(&cmpr_link, None)?
        } else {
            self.request_builder.receive_message_from_address(link)?
        };
        let mut http_client = HyperEsp32Client::new(&self.esp_http_client_opt, UserAgentName::Main);
        let response = self.request(req, &mut http_client).await?;
        self.handle_recv_message_response(response, link).await
    }

    async fn handle_recv_message_response<'a>(&mut self, response: SimpleHttpResponse, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[StreamsTransportSocketEspRs.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials using EspTimerService if needed.
        // May be StatusCode::CONTINUE is handled by the EspHttpClient
        if response.status == StatusCode::CONTINUE {
            log::warn!("[StreamsTransportSocketEspRs.recv_message_via_http] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
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
            log::debug!("[StreamsTransportSocketEspRs.recv_message_via_http] StatusCode is successful: {}", response.status);
            log::info!("[StreamsTransportSocketEspRs.recv_message_via_http] create TangleMessage ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}",
                       response.body.len(),
                       response.body.as_slice()
            );
            let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&response.body).unwrap();
            log::debug!("[StreamsTransportSocketEspRs.recv_message_via_http] return ret_val");
            Ok(ret_val)
        } else {
            log::error!("[StreamsTransportSocketEspRs.recv_message_via_http] StatusCode is not OK");
            err!(MapStreamsErrors::from_http_status_codes(
                response.status,
                Some(link.to_string())
            ))
        }
    }
}

impl CompressedStateSend for StreamsTransportSocketEspRs {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        self.compressed.subscribe_listener(listener)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        log::debug!("[StreamsTransportSocketEspRs::set_initial_use_compressed_msg_state()] use_compressed_msg is set to {}", use_compressed_msg);
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for StreamsTransportSocketEspRs
{
    async fn send_message(&mut self, msg: &TangleMessage) -> anyhow::Result<()> {
        log::info!("[StreamsTransportSocketEspRs.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> anyhow::Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> anyhow::Result<TangleMessage> {
        log::debug!("[StreamsTransportSocketEspRs.recv_message]");
        let ret_val = self.recv_message_via_http(link).await;
        log::debug!("[StreamsTransportSocketEspRs.recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[StreamsTransportSocketEspRs.recv_message] ret_val Ok");
                log::info!("[StreamsTransportSocketEspRs.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string())
            },
            Err(err) => {
                log::error!("[StreamsTransportSocketEspRs.recv_message] Received streams error: '{}'", err);
                ()
            }
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for StreamsTransportSocketEspRs {
    type Details = Details;
    async fn get_link_details(&mut self, _link: &TangleAddress) -> anyhow::Result<Self::Details> {
        unimplemented!()
    }
}

impl TransportOptions for StreamsTransportSocketEspRs {
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
