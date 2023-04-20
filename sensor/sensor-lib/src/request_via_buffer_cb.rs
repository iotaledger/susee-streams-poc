use std::{clone::Clone, slice, ptr, fmt};

use streams_tools::{
    binary_persist::{
        BinaryPersist,
        binary_persist_iota_bridge_req::{
            IotaBridgeResponseParts,
        },
    }
};

use crate::streams_poc_lib_api_types::{
    send_request_via_lorawan_t,
    StreamsError,
    LoRaWanError,
    resolve_request_response_t
};

use anyhow::{
    Result,
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

extern fn dummy_send_callback_for_httpclientoptions_default(
    _request_data: *const cty::uint8_t,
    _length: cty::size_t,
    _response_callback: resolve_request_response_t,
    _p_caller_user_data: *mut cty::c_void,
) -> LoRaWanError
{
    LoRaWanError::LORAWAN_NO_CONNECTION
}


#[derive(Clone)]
pub struct RequestViaBufferCallbackOptions {
    pub send_callback: send_request_via_lorawan_t,
    pub p_caller_user_data: *mut cty::c_void,
}

impl Default for RequestViaBufferCallbackOptions {
    fn default() -> Self {
        RequestViaBufferCallbackOptions {
            send_callback: dummy_send_callback_for_httpclientoptions_default,
            p_caller_user_data: ptr::null_mut::<cty::c_void>()
        }
    }
}

impl fmt::Display for RequestViaBufferCallbackOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RequestViaBufferCallbackOptions: send_callback defined: {}, p_caller_user_data not NUll: {}",
               true,
               self.p_caller_user_data != ptr::null_mut::<cty::c_void>(),
        )
    }
}

#[derive(Clone)]
pub struct RequestViaBufferCallback {
    send_callback: send_request_via_lorawan_t,
    p_caller_user_data: *mut cty::c_void,
}

impl RequestViaBufferCallback {
    pub fn new(options: Option<RequestViaBufferCallbackOptions>) -> Self {
        log::debug!("[RequestViaBufferCallback::new()] Unwrapping options");
        let options = options.unwrap_or_else( || RequestViaBufferCallbackOptions::default());
        log::debug!("[RequestViaBufferCallback::new()] Creating new RequestViaBufferCallback");
        Self {
            send_callback: options.send_callback,
            p_caller_user_data: options.p_caller_user_data,
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
                log::error!("[RequestViaBufferCallback - fn receive_response] Internal async channel has been closed \
        before response could been transmitted.\n Error: {}\n\
        Returning StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR", e);
                StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR
            }
        }
    } else {
        log::error!("[RequestViaBufferCallback - fn receive_response] You need to send a request before calling this function.\
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
            log::error!("[RequestViaBufferCallback - fn receive_response] You need to send a request before calling this function.");
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

impl RequestViaBufferCallback
{
    pub async fn request_via_buffer_callback(&mut self, buffer: Vec<u8>) -> Result<IotaBridgeResponseParts> {
        let _response_callback_scope_manager = ResponseCallbackScopeManager::new();
        match (self.send_callback)(buffer.as_ptr(), buffer.len(), receive_response, self.p_caller_user_data) {
            LoRaWanError::LORAWAN_OK => {
                log::debug!("[RequestViaBufferCallback.request_via_lorawan] Successfully send request via LoRaWAN");
                let receiver = get_response_receiver()?;
                match receiver.recv().await {
                    Ok(response) => {
                        log::debug!("[RequestViaBufferCallback.request_via_lorawan] Received response via LoRaWAN");
                        if response.len() > 0 {
                            match IotaBridgeResponseParts::try_from_bytes(response.as_slice()) {
                                Ok(response_parts) => {
                                    log::debug!("[RequestViaBufferCallback.request_via_lorawan] Successfully deserialized response_parts:\n{}", response_parts);
                                    if !response_parts.status_code.is_success() {
                                        let err_msg = String::from_utf8(response_parts.body_bytes.clone())
                                            .unwrap_or(String::from("Could not deserialize Error message from response Body"));
                                        log::debug!("[RequestViaBufferCallback.request] Response status is not successful: Error message is:\n{}", err_msg);
                                    }
                                    Ok(response_parts)
                                },
                                Err(e) => {
                                    log::debug!("[RequestViaBufferCallback.request_via_lorawan] Error on deserializing response_parts: {}", e );
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


// These tests need to be started as follows:
//      > cargo test --package sensor-lib --lib request_via_buffer_cb::tests --features "smol_rt tokio_test" --no-default-features -- --test-threads=1
//
#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    use hyper::http::StatusCode;

    #[tokio::test]
    async fn test_response_callback_scope_manager() {
        let data_to_send: ResponseCallbackBuffer = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let mut received_data: Option<ResponseCallbackBuffer> = None;

        {
            let _response_callback_scope_manager = ResponseCallbackScopeManager::new();
            assert_eq!(unsafe {RESPONSE_CALLBACK_SCOPE.is_some() }, true);

            let data_to_send_cloned = data_to_send.clone();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                if let Some(response_scope) = unsafe { RESPONSE_CALLBACK_SCOPE.as_ref() } {
                    assert_eq!(response_scope.sender.send(data_to_send_cloned).await, Ok(()));
                }
            });

            let response_receiver = get_response_receiver().unwrap();
            match response_receiver.recv().await {
                Ok(response) => {
                    received_data = Some(data_to_send.clone());
                    assert_eq!(response, data_to_send)
                },
                Err(e) => {
                    assert_eq!("Response receiver.recv() failed: {}", e.to_string())
                }
            }
        }

        assert_eq!(unsafe {RESPONSE_CALLBACK_SCOPE.is_none() }, true);

        assert_eq!(received_data.is_some(), true);

        if let Some(rcved_data) = received_data {
            assert_eq!(rcved_data, data_to_send);
        }
    }

    struct TestSender {
        response: IotaBridgeResponseParts
    }

    impl TestSender {
        pub fn respond(&self, response_callback: resolve_request_response_t)  -> LoRaWanError {
            let mut response_buf = Vec::<u8>::with_capacity(self.response.needed_size());
            response_buf.resize(self.response.needed_size(), 0);
            self.response.to_bytes(response_buf.as_mut_slice()).expect("Could not persist response to binary buffer");
            let response_ptr = response_buf.as_ptr();
            let streams_err = response_callback(response_ptr, response_buf.len());
            assert_eq!(streams_err, StreamsError::STREAMS_OK);
            LoRaWanError::LORAWAN_OK
        }
    }

    const TEST_REQUEST: &'static str = "This is the request";

    extern fn test_send_callback(
        request_data: *const cty::uint8_t,
        length: cty::size_t,
        response_callback: resolve_request_response_t,
        p_caller_user_data: *mut cty::c_void
    ) -> LoRaWanError {
        let request_copy = unsafe {
            slice::from_raw_parts(request_data, length)
        }.to_vec().clone();
        let request_str = String::from_utf8(request_copy).unwrap();
        assert_eq!(request_str, TEST_REQUEST);

        let test_sender: &mut TestSender = unsafe { &mut *(p_caller_user_data as *mut TestSender) };
        test_sender.respond(response_callback)
    }

    #[tokio::test]
    async fn test_request_via_buffer_callback() {
        let mut test_sender: TestSender = TestSender {
            response: IotaBridgeResponseParts {
                body_bytes: "This is the reponse".as_bytes().to_vec(),
                status_code: StatusCode::OK,
            }
        };
        let options = RequestViaBufferCallbackOptions{
            send_callback: test_send_callback,
            p_caller_user_data: &mut test_sender as *mut _ as *mut c_void,
        };

        let mut request_mngr = RequestViaBufferCallback::new(Some(options));
        let data_to_send: Vec<u8> = TEST_REQUEST.as_bytes().to_vec();
        let response_parts = request_mngr.request_via_buffer_callback(data_to_send).await
            .expect("Error while sending request via buffer callback");
        assert_eq!(response_parts, test_sender.response)
    }
}
