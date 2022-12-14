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
    http::{
        RequestBuilderStreams,
        MapStreamsErrors,
    },
    client_base::STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed
    },
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    }
};

use hyper::{
    Client as HyperClient,
    body as hyper_body,
    Body,
    client::HttpConnector,
    http::{
        StatusCode,
        Request,
        Response,
        Result as HyperResult,
    },
};

use tokio::time;

use std::{
    fmt,
    rc::Rc,
};

pub struct HttpClientOptions<'a> {
    pub http_url: &'a str,
}

impl Default for HttpClientOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL
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
    tangle_client_options: SendOptions,
    hyper_client: HyperClient<HttpConnector, Body>,
    request_builder: RequestBuilderStreams,
    compressed: CompressedStateManager,
}

impl HttpClient
{
    pub fn new(options: Option<HttpClientOptions>) -> Self {
        let options = options.unwrap_or_default();
        println!("[HttpClient.new_from_url()] Initializing instance with options:\n{}\n", options);
        Self {
            tangle_client_options: SendOptions::default(),
            hyper_client: HyperClient::new(),
            request_builder: RequestBuilderStreams::new(options.http_url),
            compressed: CompressedStateManager::new(),
        }
    }
}

impl HttpClient
{
    async fn request(&mut self, request: Request<Body>) -> Result<Response<Body>> {
        let reponse = self.hyper_client.request(request).await
            .expect("Could not build request");

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if reponse.status() == StatusCode::ALREADY_REPORTED {
            self.compressed.set_use_compressed_msg(true);
        }
        Ok(reponse)
    }

    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {
        let req = if self.compressed.get_use_compressed_msg() {
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg);
            self.request_builder.send_compressed_message(&cmpr_message)?
        } else {
            self.request_builder.send_message(msg)?
        };
        self.request(req).await?;
        Ok(())
    }

    fn get_recv_message_request(&self, link: &TangleAddress) -> HyperResult<Request<Body>> {
        if self.compressed.get_use_compressed_msg() {
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link);
            self.request_builder.receive_compressed_message_from_address(&cmpr_link)
        } else {
            self.request_builder.receive_message_from_address(link)
        }
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let req = self.get_recv_message_request(link)?;
        let mut response = self.request(req).await?;
        // TODO: This retrials are most probably not needed because they might be handled by hyper
        //       => Clarify and remove unneeded code
        if response.status() == StatusCode::CONTINUE {
            let mut interval = time::interval(Duration::from_millis(500));
            while response.status() == StatusCode::CONTINUE {
                interval.tick().await;
                response = self.request(
                    self.get_recv_message_request(link)?
                ).await?;
            }
        }

        if response.status().is_success() {
            let bytes = hyper_body::to_bytes(response.into_body()).await?;
            Ok(<TangleMessage as BinaryPersist>::try_from_bytes(&bytes).unwrap())
        } else {
            err!(MapStreamsErrors::from_http_status_codes(response.status(), Some(link.to_string())))
        }
    }
}

impl CompressedStateSend for HttpClient {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        self.compressed.subscribe_listener(listener)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for HttpClient
{
    async fn send_message(&mut self, msg: &TangleMessage) -> Result<()> {
        println!("[HttpClient.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let ret_val = self.recv_message_via_http(link).await;
        match ret_val.as_ref() {
            Ok(msg) => println!("[HttpClient.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string()),
            _ => ()
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for HttpClient {
    type Details = Details;
    async fn get_link_details(&mut self, _link: &TangleAddress) -> Result<Self::Details> {
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
