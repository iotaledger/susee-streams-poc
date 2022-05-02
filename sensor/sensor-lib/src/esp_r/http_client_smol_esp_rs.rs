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
    // time::{
    //     Duration,
    // },
};

use streams_tools::{
    RequestBuilderStreams,
    http_protocol_streams::MapStreamsErrors,
    BinaryPersist
};

use hyper::{
    // Client as HyperClient,
    body as hyper_body,
    Body,
    Request as HyperRequest,
    // ResponseFuture,
    // client::HttpConnector,       // Only available if hyper feature 'tcp' is activated
    http::{
        Method,
        StatusCode,
    }
};

use iota_client_types::{
    Details,
    SendOptions
};

#[cfg(feature = "esp_idf")]
use embedded_svc::{
//    sys_time::SystemTime,
//     timer::{
//         TimerService,
//         PeriodicTimer,
//         Timer,
//     },
    http::{
        Status,
        Headers,
        client::{
            Client,
            Request,
        },
    },
    io::Read,
};

#[cfg(feature = "esp_idf")]
use esp_idf_sys::EspError;

#[cfg(feature = "esp_idf")]
use esp_idf_svc::{
    http::client::{
        EspHttpClient,
        EspHttpResponse,
    },
// timer::{
//     EspTimerService,
//     EspTimer,
// }
};

use anyhow::{
    bail,
};

// #[cfg(feature = "esp_idf")]
// fn getIntervalTimer(duration: Duration, callback: impl FnMut() + Send + 'static) -> Result<EspTimer, EspError> {
//     let mut interval_timer = EspTimerService::new()?.timer(callback)?;
//     interval_timer.every(duration)?;
// }

pub struct HttpClientOptions<'a> {
    pub(crate) http_url: &'a str,
}

impl Default for HttpClientOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: "http://192.168.38.69:50000"
        }
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
            self.request(&mut http_client, req).await?;
        #[cfg(not(feature = "esp_idf"))]
        println!("[HttpClient.send_message_via_http] ***** self.request(&mut http_client, req) ***** ");
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        println!("[HttpClient.recv_message_via_http]");
        #[cfg(feature = "esp_idf")]
            let mut http_client = EspHttpClient::new_default()?;
        println!("[HttpClient.recv_message_via_http] EspHttpClient created");
        #[cfg(feature = "esp_idf")]
            let response: EspHttpResponse = self.request(
            &mut http_client,
            self.request_builder.receive_message_from_address(link)?,
        ).await?;
        #[cfg(not(feature = "esp_idf"))]
        println!("[HttpClient.recv_message_via_http] ***** let response: EspHttpResponse = self.request ***** ");


        println!("[HttpClient.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials using EspTimerService
        #[cfg(feature = "esp_idf")]
        if response.status() == StatusCode::CONTINUE {
            // let periodic = getPeriodicTimer(Duration::from_millis(500), move || {
            //     response = self.request(
            //             self.request_builder.receive_message_from_address(link)?
            //         ).await?;
            // });

            // let mut interval = time::interval(Duration::from_millis(500));
            // while response.status() == StatusCode::CONTINUE {
            //     interval.tick().await;
            //     response = self.request(
            //        self.request_builder.receive_message_from_address(link)?
            //     ).await?;
            // }
        }

        #[cfg(feature = "esp_idf")]
        if response.status() == StatusCode::OK {
            println!("[HttpClient.recv_message_via_http] StatusCode::OK");
            if let Some(content_len) = response.content_len() {
                println!("[HttpClient.recv_message_via_http] response.content_len()={}", content_len);
                let mut buffer = Vec::new();
                buffer.resize(content_len, 0);
                println!("[HttpClient.recv_message_via_http] do_read");
                (&response).do_read(&mut buffer)?;
                println!("[HttpClient.recv_message_via_http] create TangleMessage ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
                let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&buffer).unwrap();
                println!("[HttpClient.recv_message_via_http] return ret_val");
                Ok(ret_val)
            } else {
                println!("[HttpClient.recv_message_via_http] response.content_len() is None");
                err!(MapStreamsErrors::from_http_status_codes(
                StatusCode::from_u16(response.status())?,
                 Some(link.to_string())
            ))
            }
        } else {
            println!("[HttpClient.recv_message_via_http] MapStreamsErrors");
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
    pub async fn request<'a>(&mut self, http_client: &'a mut EspHttpClient, req: HyperRequest<Body>) -> Result<EspHttpResponse<'a>> { // Result<EspHttpResponse, EspError> {
        let svc_http_method = match req.method() {
            &Method::POST => embedded_svc::http::Method::Post,
            &Method::GET => embedded_svc::http::Method::Get,
            _ => embedded_svc::http::Method::Unbind,
        };

        let esp_http_req = http_client.request(
            svc_http_method,
            req.uri().to_string(),
        )?;

        let resulting_ret_val: Result<EspHttpResponse, EspError>;
        match req.method() {
            &Method::POST => {
                let bytes = hyper_body::to_bytes(req.into_body()).await.unwrap();
                println!("[HttpClient.request] Bytes to send: Length: {}\n    {:02X?}", bytes.len(), bytes);
                resulting_ret_val = esp_http_req.send_bytes(bytes);
            },
            &Method::GET => {
                resulting_ret_val = esp_http_req.submit();
            },
            _ => {
                bail!("Method '{}' is currently not supported", req.method())
            },
        }

        match resulting_ret_val {
            Ok(resp) => {
                println!("[HttpClient.request] Received EspHttpResponse");
                Ok(resp)
            },
            Err(e) => {
                bail!("espHttpReq.submit failed: {}", e)
            }
        }
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for HttpClient
{
    async fn send_message(&mut self, msg: &TangleMessage) -> anyhow::Result<()> {
        println!("[HttpClient.send_message] Sending message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> anyhow::Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> anyhow::Result<TangleMessage> {
        println!("[HttpClient.recv_message]");
        let ret_val = self.recv_message_via_http(link).await;
        println!("[HttpClient.recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                println!("[HttpClient.recv_message] ret_val Ok");
                println!("[HttpClient.recv_message] Receiving message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string())
            },
            _ => {
                println!("[HttpClient.recv_message] ret_val not Ok");
                ()
            }
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for HttpClient {
    type Details = Details;
    async fn get_link_details(&mut self, link: &TangleAddress) -> anyhow::Result<Self::Details> {
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
