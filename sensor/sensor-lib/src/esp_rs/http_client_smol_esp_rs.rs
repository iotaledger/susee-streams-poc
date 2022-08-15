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
    fmt
};

use streams_tools::{
    http::{
        RequestBuilderStreams,
        MapStreamsErrors,
    },
    binary_persist::BinaryPersist,
    STREAMS_TOOLS_CONST_HTTP_PROXY_URL
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

#[cfg(feature = "esp_idf")]
use embedded_svc::{
    io::Read,
    http::{
        Status,
        Headers,
    }
};

use crate::esp_rs::hyper_esp_rs_tools::send_hyper_request_via_esp_http;


#[cfg(feature = "esp_idf")]
use esp_idf_svc::{
    http::client::{
        EspHttpClient,
        EspHttpResponse,
//        EspHttpRequestWrite,
    },
};

pub struct HttpClientOptions<'a> {
    pub(crate) http_url: &'a str,
}

impl Default for HttpClientOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_HTTP_PROXY_URL
        }
    }
}

impl fmt::Display for HttpClientOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpClientOptions: http_url: {}", self.http_url)
    }
}

#[derive(Clone)]
pub struct HttpClient {
    request_builder: RequestBuilderStreams,
    tangle_client_options: SendOptions,
}

impl HttpClient
{
    pub fn new(options: Option<HttpClientOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[HttpClient::new()] Creating new HttpClient using options: {}", options);
        Self {
            request_builder: RequestBuilderStreams::new(options.http_url),
            tangle_client_options: SendOptions::default(),
        }
    }

    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {
        let req = self.request_builder.send_message(msg)?;
        #[cfg(feature = "esp_idf")]
            let mut http_client = EspHttpClient::new_default()?;
        #[cfg(feature = "esp_idf")]
            send_hyper_request_via_esp_http(&mut http_client, req).await?;
        #[cfg(not(feature = "esp_idf"))]
            log::warn!("[HttpClient.send_message_via_http] send_hyper_request_via_esp_http(&mut http_client, req) call is skipped. Enable feature 'esp_idf' to use http client.");
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[HttpClient.recv_message_via_http]");
        #[cfg(feature = "esp_idf")]
            let mut http_client = EspHttpClient::new_default()?;
        log::debug!("[HttpClient.recv_message_via_http] EspHttpClient created");
        #[cfg(feature = "esp_idf")]
            let mut response: EspHttpResponse = send_hyper_request_via_esp_http(
                &mut http_client,
                self.request_builder.receive_message_from_address(link)?,
            ).await?;
        #[cfg(not(feature = "esp_idf"))]
            log::warn!("[HttpClient.recv_message_via_http] Calling send_hyper_request_via_esp_http() is skipped. Enable feature 'esp_idf' to use http client.");


        log::debug!("[HttpClient.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials using EspTimerService if needed.
        // May be StatusCode::CONTINUE is handled by the EspHttpClient
        #[cfg(feature = "esp_idf")]
        if response.status() == StatusCode::CONTINUE {
            log::warn!("[HttpClient.recv_message_via_http] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
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

        #[cfg(feature = "esp_idf")]
        if response.status() == StatusCode::OK {
            log::debug!("[HttpClient.recv_message_via_http] StatusCode::OK");
            if let Some(content_len) = response.content_len() {
                log::info!("[HttpClient.recv_message_via_http] Received response with content length of {}", content_len);
                let mut buffer = Vec::new();
                buffer.resize(content_len, 0);
                log::debug!("[HttpClient.recv_message_via_http] read");
                (&mut response).read(&mut buffer)?;
                log::info!("[HttpClient.recv_message_via_http] create TangleMessage ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
                let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&buffer).unwrap();
                log::debug!("[HttpClient.recv_message_via_http] return ret_val");
                Ok(ret_val)
            } else {
                log::error!("[HttpClient.recv_message_via_http] response.content_len() is None");
                err!(MapStreamsErrors::from_http_status_codes(
                    StatusCode::from_u16(response.status())?,
                     Some(link.to_string())
                ))
            }
        } else {
            log::error!("[HttpClient.recv_message_via_http] StatusCode is not OK");
            err!(MapStreamsErrors::from_http_status_codes(
                StatusCode::from_u16(response.status())?,
                 Some(link.to_string())
            ))
        }
        #[cfg(not(feature = "esp_idf"))]
            let mut buffer = Vec::new();
        #[cfg(not(feature = "esp_idf"))]
            Ok(<TangleMessage as BinaryPersist>::try_from_bytes(&buffer).unwrap())
    }
}

#[cfg(feature = "esp_idf")]
impl HttpClient
{
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for HttpClient
{
    async fn send_message(&mut self, msg: &TangleMessage) -> anyhow::Result<()> {
        log::info!("[HttpClient.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> anyhow::Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> anyhow::Result<TangleMessage> {
        log::debug!("[HttpClient.recv_message]");
        let ret_val = self.recv_message_via_http(link).await;
        log::debug!("[HttpClient.recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[HttpClient.recv_message] ret_val Ok");
                log::info!("[HttpClient.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string())
            },
            Err(err) => {
                log::error!("[HttpClient.recv_message] Received streams error: '{}'", err);
                ()
            }
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for HttpClient {
    type Details = Details;
    async fn get_link_details(&mut self, _link: &TangleAddress) -> anyhow::Result<Self::Details> {
        unimplemented!()
    }
}

impl TransportOptions for HttpClient {
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
