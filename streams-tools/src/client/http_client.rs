use iota_streams::{
    app::{
        transport::{
            Transport,
            TransportDetails,
            TransportOptions,
            tangle::{
                TangleAddress,
                TangleMessage,
                client::{
                    Client,
                    Details,
                    SendOptions,
                }
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
    time::{
        Duration,
    },
};

use crate::{
    RequestBuilderStreams,
    client_base::STREAMS_TOOLS_CONST_HTTP_PROXY_URL,
    binary_persist::BinaryPersist,
    http_protocol_streams::MapStreamsErrors,
};

use hyper::{
    Client as HyperClient,
    body as hyper_body,
    Body,
    client::HttpConnector,
    http::StatusCode,
};

use tokio::time;
use std::fmt;

pub struct HttpClientOptions<'a> {
    pub http_url: &'a str,
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
    client: Client,
    hyper_client: HyperClient<HttpConnector, Body>,
    request_builder: RequestBuilderStreams,
}

impl HttpClient
{
    pub fn new_from_url(url: &str, options: Option<HttpClientOptions>) -> Self {
        let options = options.unwrap_or_default();
        println!("[HttpClient.new_from_url()] Initializing instance with options:\n{}\n", options);
        Self {
            client: Client::new_from_url(url),
            hyper_client: HyperClient::new(),
            request_builder: RequestBuilderStreams::new(options.http_url)
        }
    }
}

impl HttpClient
{
    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {
        let req = self.request_builder.send_message(msg)?;
        self.hyper_client.request(req).await?;
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let mut response = self.hyper_client.request(
            self.request_builder.receive_message_from_address(link)?
        ).await?;
        // TODO: This retrials are most probably not needed because they might be handled by hyper
        //       => Clarify and remove unneeded code
        if response.status() == StatusCode::CONTINUE {
            let mut interval = time::interval(Duration::from_millis(500));
            while response.status() == StatusCode::CONTINUE {
                interval.tick().await;
                response = self.hyper_client.request(
                   self.request_builder.receive_message_from_address(link)?
                ).await?;
            }
        }

        if response.status() == StatusCode::OK {
            let bytes = hyper_body::to_bytes(response.into_body()).await?;
            Ok(<TangleMessage as BinaryPersist>::try_from_bytes(&bytes).unwrap())
        } else {
            err!(MapStreamsErrors::from_http_status_codes(response.status(), Some(link.to_string())))
        }
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for HttpClient
{
    async fn send_message(&mut self, msg: &TangleMessage) -> Result<()> {
        println!("[HttpClient.send_message] Sending message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let ret_val = self.recv_message_via_http(link).await;
        match ret_val.as_ref() {
            Ok(msg) => println!("[HttpClient.recv_message] Receiving message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string()),
            _ => ()
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for HttpClient {
    type Details = Details;
    async fn get_link_details(&mut self, link: &TangleAddress) -> Result<Self::Details> {
        self.client.get_link_details(link).await
    }
}

impl TransportOptions for HttpClient {
    type SendOptions = SendOptions;
    fn get_send_options(&self) -> SendOptions {
        self.client.get_send_options()
    }
    fn set_send_options(&mut self, opt: SendOptions) {
        self.client.set_send_options(opt)
    }

    type RecvOptions = ();
    fn get_recv_options(&self) {}
    fn set_recv_options(&mut self, _opt: ()) {}
}
