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
    },
};

use std::{
    marker::PhantomData,
    clone::Clone,
    time::{
        Instant,
        Duration,
    },
};

use crate::{
    RequestBuilder,
    client_base::STREAMS_TOOLS_CONST_HTTP_PROXY_URL,
    binary_persistence::BinaryPersist
};

use hyper::{
    Client as HyperClient,
    body as hyper_body,
    Body,
    client::HttpConnector,
};

use tokio::time;
use hyper::http::StatusCode;

pub struct HttpClientOptions<'a> {
    http_url: &'a str,
}

impl Default for HttpClientOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_HTTP_PROXY_URL
        }
    }
}


#[derive(Clone)]
pub struct HttpClient<F> {
    _phantom: PhantomData<F>,
    client: Client,
    hyper_client: HyperClient<HttpConnector, Body>,
    request_builder: RequestBuilder,
}

impl<F> HttpClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    pub fn new_from_url(url: &str, options: Option<HttpClientOptions>) -> Self {
        let options = options.unwrap_or_default();
        Self {
            _phantom: PhantomData,
            client: Client::new_from_url(url),
            hyper_client: HyperClient::new(),
            request_builder: RequestBuilder::new(options.http_url)
        }
    }
}

impl<F> HttpClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    async fn send_message_via_http(&mut self, msg: &TangleMessage<F>) -> Result<()> {
        let req = self.request_builder.send_message(msg)?;
        self.hyper_client.request(req).await?;
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage<F>> {
        let mut response = self.hyper_client.request(
            self.request_builder.receive_message_from_address(link)?
        ).await?;
        if response.status() == StatusCode::CONTINUE {
            let mut interval = time::interval(Duration::from_millis(500));
            while response.status() == StatusCode::CONTINUE {
                interval.tick().await;
                response = self.hyper_client.request(
                    self.request_builder.receive_message_from_address(link)?
                ).await?;
            }
        }

        let bytes = hyper_body::to_bytes(response.into_body()).await?;
        Ok(<TangleMessage<F> as BinaryPersist>::try_from_bytes(&bytes).unwrap())
    }
}

#[async_trait(?Send)]
impl<F> Transport<TangleAddress, TangleMessage<F>> for HttpClient<F>
    where
        F: 'static + core::marker::Send + core::marker::Sync,
{
    async fn send_message(&mut self, msg: &TangleMessage<F>) -> Result<()> {
        println!("[HttpClient.send_message] Sending message with {} bytes payload:\n{}\n", msg.binary.body.bytes.len(), msg.binary.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, link: &TangleAddress) -> Result<Vec<TangleMessage<F>>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage<F>> {
        let ret_val = self.recv_message_via_http(link).await;
        match ret_val.as_ref() {
            Ok(msg) => println!("[HttpClient.recv_message] Receiving message with {} bytes payload:\n{}\n", msg.binary.body.bytes.len(), msg.binary.to_string()),
            _ => ()
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl<F> TransportDetails<TangleAddress> for HttpClient<F> {
    type Details = Details;
    async fn get_link_details(&mut self, link: &TangleAddress) -> Result<Self::Details> {
        self.client.get_link_details(link).await
    }
}

impl<F> TransportOptions for HttpClient<F> {
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
