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
    fmt,
    rc::Rc,
    time::{
        Duration,
    },
};

use crate::{
    http::{
        RequestBuilderStreams,
        MapStreamsErrors,
        http_tools::RequestBuilderTools,
        http_protocol_lorawan_rest::RequestBuilderLorawanRest,
        http_protocol_streams::{
            EndpointUris,
            QueryParameters
        }
    },
    client_base::STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            IotaBridgeResponseParts
        }
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
    },
};

use tokio::time;

use anyhow::bail;

pub struct HttpClientOptions<'a> {
    pub http_url: &'a str,
    pub dev_eui: Option<String>,
    pub use_lorawan_rest: bool,
}

impl<'a> HttpClientOptions<'a> {
    pub fn new(http_url: &'a str) -> Self {
        let mut ret_val = Self::default();
        ret_val.http_url = http_url;
        ret_val
    }
}

impl Default for HttpClientOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
            dev_eui: None,
            use_lorawan_rest: false,
        }
    }
}

impl fmt::Display for HttpClientOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HttpClientOptions:\n     http_url: {},\n     dev_eui:  {},\n     use_lorawan_rest:  {}",
            self.http_url,
            if let Some(eui) = &self.dev_eui {eui.as_str()} else {""},
            self.use_lorawan_rest
        )
    }
}

#[derive(Clone)]
pub struct HttpClient {
    tangle_client_options: SendOptions,
    hyper_client: HyperClient<HttpConnector, Body>,
    request_builder_streams: RequestBuilderStreams,
    request_builder_lorawan_rest: RequestBuilderLorawanRest,
    compressed: CompressedStateManager,
    dev_eui: Option<String>,
    use_lorawan_rest: bool,
}

impl HttpClient
{
    pub fn new(options: Option<HttpClientOptions>) -> Self {
        let options = options.unwrap_or_default();
        println!("[HttpClient.new_from_url()] Initializing instance with options:\n{}\n", options);
        Self {
            tangle_client_options: SendOptions::default(),
            hyper_client: HyperClient::new(),
            request_builder_streams: RequestBuilderStreams::new(options.http_url),
            request_builder_lorawan_rest: RequestBuilderLorawanRest::new(options.http_url),
            compressed: CompressedStateManager::new(),
            dev_eui: options.dev_eui,
            use_lorawan_rest: options.use_lorawan_rest
        }
    }
}

impl HttpClient
{
    fn get_lorawan_rest_request(&self, req_parts: IotaBridgeRequestParts) -> Result<Request<Body>>{
        let mut buffer: Vec<u8> = vec![0; req_parts.needed_size()];
        req_parts.to_bytes(buffer.as_mut_slice())?;
        if let Some(dev_eui) = self.dev_eui.as_ref() {
            Ok(self.request_builder_lorawan_rest.post_binary_request(buffer, dev_eui.as_str())
                .expect("Error on creating hyper request for lorawan-rest/post_binary_request call")
            )
        } else {
            bail!("You need to specify a dev_eui in the HttpClientOptions to use the lorawan-rest API with this HttpClient" )
        }
    }

    async fn request(&mut self, req_parts: IotaBridgeRequestParts) -> Result<Response<Body>> {
        let request = if self.use_lorawan_rest {
            self.get_lorawan_rest_request(req_parts)?
        } else {
            req_parts.into_request(RequestBuilderTools::get_request_builder())?
        };

        let mut response = self.hyper_client.request(request).await
            .expect("Error while sending request via hyper_client");

        if self.use_lorawan_rest {
            response = HttpClient::handle_lorawan_rest_response(response).await?;
        }

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response.status() == StatusCode::ALREADY_REPORTED {
            self.compressed.set_use_compressed_msg(true);
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
                        log::debug!("[HttpClient.request] Successfully deserialized response_parts:\n{}", response_parts);
                        if !response_parts.status_code.is_success() {
                            let err_msg = String::from_utf8(response_parts.body_bytes.clone())
                                .unwrap_or(String::from("Could not deserialize Error message from response Body"));
                            log::debug!("[HttpClient.request] Response status is not successful: Error message is:\n{}", err_msg);
                        }
                        ret_val = Response::builder()
                            .status(response_parts.status_code)
                            .body(Body::from(response_parts.body_bytes))?;
                    },
                    Err(e) => {
                        log::debug!("[HttpClient.request] Error on deserializing response_parts: {}", e);
                        bail!("Could not deserialize response binary to valid IotaBridgeResponseParts: {}", e)
                    }
                }
            } else {
                bail!("Received 0 bytes response from server.")
            }
        } else {
            log::error!("[HttpClient.request] Received HTTP Error from Iota-Bridge. Status: {}", ret_val.status());
            log::error!("[HttpClient.request] Returning original lorawan-rest response");
        }
        Ok(ret_val)
    }

    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // In contrast to http_client_lorawan::HttpClient we set the dev_eui here because it could be
            // used in cases where the lorawan-rest API is noz used for compressed messages.
            // http_client_lorawan::HttpClient never sets the DevEUI because it is communicated by the
            // LoraWAN network automatically (compare comment in function HttpClient::recv_message_via_http()
            // in sensor/sensor-lib/src/esp_rs/streams_poc_lib/http_client_lorawan.rs).
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg);
            self.request_builder_streams.get_send_message_request_parts(&cmpr_message, EndpointUris::SEND_COMPRESSED_MESSAGE, true, self.dev_eui.clone())?
        } else {
            self.request_builder_streams.get_send_message_request_parts(msg, EndpointUris::SEND_MESSAGE, false, None)?
        };
        self.request(req_parts).await?;
        Ok(())
    }

    fn get_recv_message_request(&self, link: &TangleAddress) -> Result<IotaBridgeRequestParts> {
        let ret_val = if self.compressed.get_use_compressed_msg() {
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link);
            self.request_builder_streams.get_receive_message_from_address_request_parts(
                &cmpr_link,
                EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
                true,
                QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID,
                self.dev_eui.clone(),
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
        println!("[HttpClient.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n",
                 msg.body.as_bytes().len() as u32, msg.body.to_string());
        self.send_message_via_http(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let ret_val = self.recv_message_via_http(link).await;
        match ret_val.as_ref() {
            Ok(msg) => println!("[HttpClient.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n",
                                msg.body.as_bytes().len() as u32, msg.body.to_string()),
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
