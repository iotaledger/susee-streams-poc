use iota_streams::{
    app::{
        transport::{
            Transport,
            TransportDetails,
            TransportOptions,
            tangle::{
                TangleAddress,
                TangleMessage,
                AppInst
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
    rc::Rc,
    ptr
};

use streams_tools::{
    http::{
        RequestBuilderStreams,
        http_protocol_streams::{
            MapStreamsErrors,
            EndpointUris,
            QueryParameters,
        },
    },
    binary_persist::{
        BinaryPersist,
        TangleMessageCompressed,
        TangleAddressCompressed,
        binary_persist_iota_bridge_req::{
            IotaBridgeRequestParts,
            IotaBridgeResponseParts,
        },
    },
    compressed_state::{
        CompressedStateSend,
        CompressedStateListen,
        CompressedStateManager
    },
};

use crate::streams_poc_lib::api_types::{
    send_request_via_lorawan_t,
    StreamsError,
    LoRaWanError,
    resolve_request_response_t
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

use anyhow::{
    bail,
};

use smol::channel::{
    bounded,
    Sender,
    Receiver,
};

use futures_lite::future;

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
    _response_callback: resolve_request_response_t,
    _p_caller_user_data: *mut cty::c_void,
) -> LoRaWanError
{
    LoRaWanError::LORAWAN_NO_CONNECTION
}


pub struct HttpClientViaBufferCallbackOptions {
    pub lorawan_send_callback: send_request_via_lorawan_t,
    pub p_caller_user_data: *mut cty::c_void,
}

impl Default for HttpClientViaBufferCallbackOptions {
    fn default() -> Self {
        HttpClientViaBufferCallbackOptions {
            lorawan_send_callback: dummy_lorawan_send_callback_for_httpclientoptions_default,
            p_caller_user_data: ptr::null_mut::<cty::c_void>()
        }
    }
}

#[derive(Clone)]
pub struct HttpClientViaBufferCallback {
    lorawan_send_callback: send_request_via_lorawan_t,
    request_builder: RequestBuilderStreams,
    tangle_client_options: SendOptions,
    compressed: CompressedStateManager,
    p_caller_user_data: *mut cty::c_void,
}

impl HttpClientViaBufferCallback {
    pub fn new(options: Option<HttpClientViaBufferCallbackOptions>) -> Self {
        log::debug!("[HttpClientViaBufferCallback::new()] Unwrapping options");
        let options = options.unwrap_or_else( || HttpClientViaBufferCallbackOptions::default());
        log::debug!("[HttpClientViaBufferCallback::new()] Creating new HttpClientViaBufferCallback");
        Self {
            lorawan_send_callback: options.lorawan_send_callback,
            p_caller_user_data: options.p_caller_user_data,
            request_builder: RequestBuilderStreams::new(""),
            tangle_client_options: SendOptions::default(),
            compressed: CompressedStateManager::new(),
        }
    }

    async fn send_message_via_lorawan(&mut self, msg: &TangleMessage) -> Result<()> {
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // Please note the comments in fn recv_message_via_http() below
            // Same principles apply here
            let cmpr_message = TangleMessageCompressed::from_tangle_message(msg);
            self.request_builder.get_send_message_request_parts(&cmpr_message, EndpointUris::SEND_COMPRESSED_MESSAGE, true, None)?
        } else {
            self.request_builder.get_send_message_request_parts(msg, EndpointUris::SEND_MESSAGE, false, None)?
        };

        self.request(req_parts, msg.link.appinst).await?;
        Ok(())
    }

    async fn recv_message_via_lorawan(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        log::debug!("[HttpClientViaBufferCallback.recv_message_via_http]");
        let req_parts = if self.compressed.get_use_compressed_msg() {
            // We do not set the dev_eui here because it will be communicated by the LoraWAN network
            // and therefore will not be send as lorawan payload.
            // Please note that due to this BinaryPersist implementation for TangleMessageCompressed
            // does not serialize/deserialize the dev_eui in general.
            let cmpr_link = TangleAddressCompressed::from_tangle_address(link);
            self.request_builder.get_receive_message_from_address_request_parts(
                &cmpr_link,
                EndpointUris::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS,
                true,
                QueryParameters::RECEIVE_COMPRESSED_MESSAGE_FROM_ADDRESS_MSGID,
                None
            )?
        } else {
            self.request_builder.get_receive_message_from_address_request_parts(
                link,
                EndpointUris::RECEIVE_MESSAGE_FROM_ADDRESS,
                false,
                QueryParameters::RECEIVE_MESSAGE_FROM_ADDRESS,
                None
            )?
        };

        let response = self.request(req_parts, link.appinst).await?;

        log::debug!("[HttpClientViaBufferCallback.recv_message_via_http] check for retrials");
        // TODO: Implement following retrials for bad LoRaWAN connection using EspTimerService if needed.
        // May be we need to introduce StatusCode::CONTINUE in cases where LoRaWAN connection
        // is sometimes too bad and retries are a valid strategy to receive the response
        if response.status_code == StatusCode::CONTINUE {
            log::warn!("[HttpClientViaBufferCallback.recv_message_via_http] Received StatusCode::CONTINUE. Currently no retries implemented. Possible loss of data.")
        }

        if response.status_code.is_success() {
            log::debug!("[HttpClientViaBufferCallback.recv_message_via_http] StatusCode is successful: {}", response.status_code);
            log::info!("[HttpClientViaBufferCallback.recv_message_via_http] Received response with content length of {}", response.body_bytes.len());
            let ret_val = <TangleMessage as BinaryPersist>::try_from_bytes(&response.body_bytes.as_slice()).unwrap();
            log::debug!("[HttpClientViaBufferCallback.recv_message_via_http] return ret_val");
            Ok(ret_val)
        } else {
            log::error!("[HttpClientViaBufferCallback.recv_message_via_http] StatusCode is not OK");
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
                log::error!("[HttpClientViaBufferCallback - fn receive_response] Internal async channel has been closed \
        before response could been transmitted.\n Error: {}\n\
        Returning StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR", e);
                StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR
            }
        }
    } else {
        log::error!("[HttpClientViaBufferCallback - fn receive_response] You need to send a request before calling this function.\
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
            log::error!("[HttpClientViaBufferCallback - fn receive_response] You need to send a request before calling this function.");
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

impl HttpClientViaBufferCallback
{
    pub async fn request<'a>(&mut self, req_parts: IotaBridgeRequestParts, channel_id: AppInst) -> Result<IotaBridgeResponseParts> {
        let buffer = Self::binary_persist_request(req_parts)?;
        log::debug!("[HttpClientViaBufferCallback.request] IotaBridgeRequestParts bytes to send: Length: {}\n    {:02X?}", buffer.len(), buffer);
        let mut response_parts = self.request_via_lorawan(buffer).await?;
        log::debug!("[HttpClientViaBufferCallback::request()] response_parts.status_code is {}", response_parts.status_code);

        // We send uncompressed messages until we receive a 208 - ALREADY_REPORTED
        // http status which indicates that the iota-bridge has stored all needed
        // data to use compressed massages further on.
        if response_parts.status_code == StatusCode::ALREADY_REPORTED {
            log::info!("[HttpClientViaBufferCallback::request()] Received StatusCode::ALREADY_REPORTED (208)- Set use_compressed_msg = true");
            self.compressed.set_use_compressed_msg(true);
        }
        if response_parts.status_code == StatusCode::UNPROCESSABLE_ENTITY {
            response_parts = self.handle_request_retransmit(response_parts, channel_id).await?;
        }

        log::info!("[HttpClientViaBufferCallback::request()] use_compressed_msg = '{}'", self.compressed.get_use_compressed_msg());
        Ok(response_parts)
    }

    fn binary_persist_request(req_parts: IotaBridgeRequestParts) -> Result<Vec<u8>> {
        let mut buffer: Vec<u8> = vec![0; req_parts.needed_size()];
        req_parts.to_bytes(buffer.as_mut_slice())?;
        Ok(buffer)
    }

    async fn handle_request_retransmit(&mut self, mut response_parts: IotaBridgeResponseParts, channel_id: AppInst) -> Result<IotaBridgeResponseParts> {
        let mut retransmit_request = self.request_builder.retransmit(
            channel_id,
            &response_parts.body_bytes
        )?;
        log::info!("[HttpClientViaBufferCallback::handle_request_retransmit()] Received StatusCode::UNPROCESSABLE_ENTITY (422) - Processing {}",
            retransmit_request.uri());

        let retransmit_req_parts = IotaBridgeRequestParts::from_request(retransmit_request, false).await;
        let retransmit_req_bytes = Self::binary_persist_request(retransmit_req_parts)?;
        response_parts = self.request_via_lorawan(retransmit_req_bytes).await?;

        if response_parts.status_code != StatusCode::ALREADY_REPORTED {
            log::warn!("[HttpClientViaBufferCallback.handle_request_retransmit] Expected retransmit response with status 208-ALREADY_REPORTED. Got status {}", response_parts.status_code);
            log::warn!("[HttpClientViaBufferCallback.handle_request_retransmit] Will set use_compressed_msg to false for security reasons");
            self.compressed.set_use_compressed_msg(false);
        }

        Ok(response_parts)
    }
    pub async fn request_via_lorawan(&mut self, buffer: Vec<u8>) -> Result<IotaBridgeResponseParts> {
        let _response_callback_scope_manager = ResponseCallbackScopeManager::new();
        match (self.lorawan_send_callback)(buffer.as_ptr(), buffer.len(), receive_response, self.p_caller_user_data) {
            LoRaWanError::LORAWAN_OK => {
                log::debug!("[HttpClientViaBufferCallback.request_via_lorawan] Successfully send request via LoRaWAN");
                let receiver = get_response_receiver()?;
                match receiver.recv().await {
                    Ok(response) => {
                        log::debug!("[HttpClientViaBufferCallback.request_via_lorawan] Received response via LoRaWAN");
                        if response.len() > 0 {
                            match IotaBridgeResponseParts::try_from_bytes(response.as_slice()) {
                                Ok(response_parts) => {
                                    log::debug!("[HttpClientViaBufferCallback.request_via_lorawan] Successfully deserialized response_parts:\n{}", response_parts);
                                    if !response_parts.status_code.is_success() {
                                        let err_msg = String::from_utf8(response_parts.body_bytes.clone())
                                            .unwrap_or(String::from("Could not deserialize Error message from response Body"));
                                        log::debug!("[HttpClientViaBufferCallback.request] Response status is not successful: Error message is:\n{}", err_msg);
                                    }
                                    Ok(response_parts)
                                },
                                Err(e) => {
                                    log::debug!("[HttpClientViaBufferCallback.request_via_lorawan] Error on deserializing response_parts: {}", e );
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
            LoRaWanError::EXIT_SENSOR_MANAGER => {
                // TODO: Implement clean shutdown of the sensor_manager starting from here
                bail!("lorawan_send_callback returned error EXIT_SENSOR_MANAGER - clean shutdown of the sensor_manager is missing")
            }
        }
    }
}

impl CompressedStateSend for HttpClientViaBufferCallback {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        self.compressed.subscribe_listener(listener)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        log::debug!("[HttpClientViaBufferCallback::set_initial_use_compressed_msg_state()] use_compressed_msg is set to {}", use_compressed_msg);
        self.compressed.set_initial_use_compressed_msg_state(use_compressed_msg)
    }
}

#[async_trait(?Send)]
impl Transport<TangleAddress, TangleMessage> for HttpClientViaBufferCallback
{
    async fn send_message(&mut self, msg: &TangleMessage) -> anyhow::Result<()> {
        log::info!("[HttpClientViaBufferCallback.send_message] Sending message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.send_message_via_lorawan(msg).await
    }

    async fn recv_messages(&mut self, _link: &TangleAddress) -> anyhow::Result<Vec<TangleMessage>> {
        unimplemented!()
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> anyhow::Result<TangleMessage> {
        log::debug!("[HttpClientViaBufferCallback.recv_message]");
        let ret_val = self.recv_message_via_lorawan(link).await;
        log::debug!("[HttpClientViaBufferCallback.recv_message] ret_val received");
        match ret_val.as_ref() {
            Ok(msg) => {
                log::debug!("[HttpClientViaBufferCallback.recv_message] ret_val Ok");
                log::info!("[HttpClientViaBufferCallback.recv_message] Receiving message with {} bytes tangle-message-payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string())
            },
            Err(err) => {
                log::error!("[HttpClientViaBufferCallback.recv_message] Received streams error: '{}'", err);
                ()
            }
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for HttpClientViaBufferCallback {
    type Details = Details;
    async fn get_link_details(&mut self, _link: &TangleAddress) -> anyhow::Result<Self::Details> {
        unimplemented!()
    }
}

impl TransportOptions for HttpClientViaBufferCallback {
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

/* Ccurrently, this can not be compiled because of the esp-idf-sys dependency

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_response_callback_scope_manager() {
        const DATA_TO_SEND: ResponseCallbackBuffer = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let mut received_data: Some<ResponseCallbackBuffer> = None;

        {
            let _response_callback_scope_manager = ResponseCallbackScopeManager::new();
            let response_receiver = get_response_receiver().unwrap();
            match receiver.recv().await {
                Ok(response) => {
                    received_data = Some(DATA_TO_SEND.clone());
                    assert_eq!(response, DATA_TO_SEND)
                },
                Err(e) => {
                    assert_eq!("Response receiver.recv() failed: {}", response)
                }
            }
            if let Some(response_scope) = unsafe { RESPONSE_CALLBACK_SCOPE.as_ref() } {
                assert_eq!(response_scope.sender.send(DATA_TO_SEND).await, Ok(()));
            }
        }

        assert_eq!(received_data, DATA_TO_SEND);
        assert_eq!(unsafe {RESPONSE_CALLBACK_SCOPE.is_none() }, true);
    }
}
*/
