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
    slice,
};

use streams_tools::{
    http::{
        RequestBuilderStreams,
        http_protocol_streams::MapStreamsErrors,
    },
    binary_persist::{
        BinaryPersist,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
        },
    },
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

use crate::streams_poc_lib::api_types::{send_request_via_lorawan_t, StreamsError, LoRaWanError, resolve_request_response_t};

use anyhow::{
    bail,
};

use smol::channel::{
    bounded,
    Sender,
    Receiver,
};
use streams_tools::binary_persist::binary_persist_iota_bridge_req::IotaBridgeResponseParts;
use futures_lite::future;
use streams_tools::http::http_protocol_streams::{EndpointUris, QueryParameters};
use streams_tools::binary_persist::{TangleMessageCompressed, TangleAddressCompressed};

pub type ResponseCallbackBuffer = Vec<u8>;
pub type ResponseCallbackSender = Sender<ResponseCallbackBuffer>;
pub type ResponseCallbackReceiver = Receiver<ResponseCallbackBuffer>;

pub struct ResponseCallbackScope {
    pub sender: ResponseCallbackSender,
    pub receiver: ResponseCallbackReceiver,
}

impl ResponseCallbackScope {
    pub fn new() -> Self {
        let (sender, receiver) = bounded::<ResponseCallbackBuffer>(1);
        Self{
            sender,
            receiver,
        }
    }
}

// Usually we would use a thread safe shared Vec of ResponseCallbackScope instances
// to manage concurrent multiple Request -> Response transactions coming from multiple
// threads etc.
// The indx of the Vec would be provided to the user of the C binding api function
// as 'u32 handle' parameter (or similar).
// This would be a thread safe variant of:
//
//          static mut RESPONSE_CALLBACK_SCOPES: Option<Vec<ResponseCallbackScope>> = None;
//
// As the SUSEE sensor does only one transaction per time we don't need this and will
// instead return an error in case someone tries to send multiple transactions at time.
// Therefor we can have just one RESPONSE_CALLBACK_SCOPE

static mut RESPONSE_CALLBACK_SCOPE: Option<ResponseCallbackScope> = None;

extern fn dummy_lorawan_send_callback_for_httpclientoptions_default (
    _request_data: *const cty::uint8_t,
    _length: cty::size_t,
    _response_callback: resolve_request_response_t) -> LoRaWanError
{
    LoRaWanError::LORAWAN_NO_CONNECTION
}


pub struct HttpClientOptions {
    pub lorawan_send_callback: send_request_via_lorawan_t,
}

impl Default for HttpClientOptions {
    fn default() -> Self {
        HttpClientOptions {
            lorawan_send_callback: dummy_lorawan_send_callback_for_httpclientoptions_default,
        }
    }
}

#[derive(Clone)]
pub struct HttpClient {
    lorawan_send_callback: send_request_via_lorawan_t,
    request_builder: RequestBuilderStreams,
    tangle_client_options: SendOptions,
}

impl HttpClient {
    pub fn new(options: Option<HttpClientOptions>) -> Self {
        log::debug!("[HttpClient::new()] Unwrapping options");
        let options = options.unwrap_or_else( || HttpClientOptions::default());
        log::debug!("[HttpClient::new()] Creating new HttpClient");
        Self {
            lorawan_send_callback: options.lorawan_send_callback,
            request_builder: RequestBuilderStreams::new(""),
            tangle_client_options: SendOptions::default(),
        }
    }

    async fn send_message_via_http(&mut self, msg: &TangleMessage) -> Result<()> {

        // In the current implementation we always send compressed messages.
        // This implies that the IOTA Bridge already knows the streams channel_id
        // for our dev_eui.
        // In a test scenario where the same iota-bridge is used for channel initialization
        // and send-message usecases this will always be OK because the IOTA Bridge learns
        // the dev_eui+channel_id pair when the channel is initialized.
        //
        // TODO: In case multiple IOTA Bridge instances are used for these usecases
        //       the http protocol needs to handle this.
        let compressed_message = TangleMessageCompressed::from_tangle_message(msg);
        // We do not set the dev_eui here because it will be communicated by the LoraWAN network
        // and therefore will not be send as lorawan payload.
        // Please note that due to this BinaryPersist implementation for TangleMessageCompressed
        // does not serialize/deserialize the dev_eui in general.
        let req_parts = self.request_builder.get_send_message_request_parts(&compressed_message, EndpointUris::SEND_COMPRESSED_MESSAGE)?;
        self.request(req_parts).await?;
        Ok(())
    }

    async fn recv_message_via_http(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[HttpClient.recv_message_via_http]");
        // Please note the comments in fn send_message_via_http() above
        // Same principles apply here
        let compressed_tangle_address = TangleAddressCompressed::from_tangle_address(link);
        let response = self.request(
            self.request_builder.get_receive_message_from_address_request_parts(
                &compressed_tangle_address,
                EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
                QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID
            )?,
        ).await?;

        log::debug!("[HttpClient.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials for bad LoRaWAN connection using EspTimerService if needed.
        // May be we need to introduce StatusCode::CONTINUE in cases where LoRaWAN connection
        // is sometimes too bad and retries are a valid strategy to receive the response
        if response.status_code == StatusCode::CONTINUE {
            log::warn!("[HttpClient.recv_message_via_http] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
        }

        if response.status_code == StatusCode::OK {
            log::info!("[HttpClient.recv_message_via_http] Received response with content length of {}", response.body_bytes.len());
            let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&response.body_bytes.as_slice()).unwrap();
            log::debug!("[HttpClient.recv_message_via_http] return ret_val");
            Ok(ret_val)
        } else {
            log::error!("[HttpClient.recv_message_via_http] StatusCode is not OK");
            err!(MapStreamsErrors::from_http_status_codes(
                response.status_code,
                 Some(link.to_string())
            ))
        }
    }
}

pub extern "C" fn receive_response(response_data: *const cty::uint8_t, length: cty::size_t) -> StreamsError {
    // TODO: This unsafe code needs to be replaced by a thread safe shared RESPONSE_CALLBACK_SCOPE
    //       access based on Arc::new(Mutex::new(......)) as been described here
    //       https://stackoverflow.com/questions/60996488/passing-additional-state-to-rust-hyperserviceservice-fn
    if let Some(response_scope) = unsafe { RESPONSE_CALLBACK_SCOPE.as_ref() } {
        let response_copy = unsafe {
                slice::from_raw_parts(response_data, length)
            }.to_vec().clone();
        match future::block_on(response_scope.sender.send(response_copy)) {
            Ok(_) => StreamsError::STREAMS_OK,
            Err(e) => {
                log::error!("[HttpClient - fn receive_response] Internal async channel has been closed \
        before response could been transmitted.\n Error: {}\n\
        Returning StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR", e);
                StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR
            }
        }
    } else {
        log::error!("[HttpClient - fn receive_response] You need to send a request before calling this function.\
        Returning StreamsError::STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST");
        StreamsError::STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST
    }
}


pub fn get_response_receiver<'a>() -> Result<&'a ResponseCallbackReceiver> {
    unsafe {
        // TODO: This unsafe code needs to be replaced by a thread safe version. See respective comment above.
        if let Some(response_scope) = RESPONSE_CALLBACK_SCOPE.as_ref() {
            Ok(&response_scope.receiver)
        } else {
            log::error!("[HttpClient - fn receive_response] You need to send a request before calling this function.");
            bail!("Attempt to response before sending a request or echoed (doubled) response for a previous request.")
        }
    }
}


#[allow(dead_code)]
struct ResponseCallbackScopeManager<'a> {
    pub scope: &'a ResponseCallbackScope
}

impl<'a> ResponseCallbackScopeManager<'a> {
    pub fn new() -> Result<Self> {
        let borrowed_scope: &'a ResponseCallbackScope;
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe version. See respective comment above.
            if RESPONSE_CALLBACK_SCOPE.is_none() {
                RESPONSE_CALLBACK_SCOPE = Some(ResponseCallbackScope::new());
                borrowed_scope = RESPONSE_CALLBACK_SCOPE.as_ref().unwrap();
            } else {
                log::error!("[ResponseCallbackScopeManager.new] There is already a pending request send via LoRaWAN.\
                You need to wait until this request returns before you can send another request.");
                bail!("Attempt to send multiple overlapping requests at time. Only one transaction at time allowed.")
            }
        }
        Ok(Self{scope: borrowed_scope})
    }
}

impl<'a> Drop for ResponseCallbackScopeManager<'a> {
    fn drop(&mut self) {
        unsafe {
            // TODO: This unsafe code needs to be replaced by a thread safe version. See respective comment above.
            if RESPONSE_CALLBACK_SCOPE.is_some() {
                RESPONSE_CALLBACK_SCOPE = None;
            } else {
                log::warn!("[ResponseCallbackScopeManager.drop] There is no existing RESPONSE_CALLBACK_SCOPE to delete.
                Do only create or delete a RESPONSE_CALLBACK_SCOPE using a ResponseCallbackScopeManager
                instance in you fn scope to avoid errors in RESPONSE_CALLBACK_SCOPE management.");
            }
        }
    }
}

impl HttpClient
{
    pub async fn request<'a>(&mut self, req_parts: IotaBridgeRequestParts) -> Result<IotaBridgeResponseParts> {

        let _response_callback_scope_manager = ResponseCallbackScopeManager::new();

        let mut buffer: Vec<u8> = vec![0; req_parts.needed_size()];
        req_parts.to_bytes(buffer.as_mut_slice())?;
        log::debug!("[HttpClient.request] IotaBridgeRequestParts bytes to send: Length: {}\n    {:02X?}", buffer.len(), buffer);

        let success= (self.lorawan_send_callback)(buffer.as_ptr(), buffer.len(), receive_response);

        match success {
            LoRaWanError::LORAWAN_OK => {
                log::debug!("[HttpClient.request] Successfully send request via LoRaWAN");
                let receiver = get_response_receiver()?;
                match receiver.recv().await {
                    Ok(response) => {
                        log::debug!("[HttpClient.request] Received response via LoRaWAN");
                        if response.len() > 0 {
                            match IotaBridgeResponseParts::try_from_bytes(response.as_slice()) {
                                Ok(response_parts) => {
                                    log::debug!("[HttpClient.request] Successfully deserialized response_parts");
                                    Ok(response_parts)
                                },
                                Err(e) => {
                                    log::debug!("[HttpClient.request] Error on deserializing response_parts: {}", e );
                                    bail!("Could not deserialize response binary to valid IotaBridgeResponseParts: {}", e)
                                }
                            }
                        } else {
                            bail!("Received 0 bytes response from server. Connection has been shut down (shutdown(Write)).")
                        }
                    },
                    Err(e) => {
                        bail!("Response receiver.recv() failed: {}", e)
                    }
                }
            },
            LoRaWanError::LORAWAN_NO_CONNECTION => {
                bail!("lorawan_send_callback returned error LORAWAN_NO_CONNECTION")
            },
        }
    }
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
