use log::*;
use sensor_lib::{
    process_main_esp_rs,
    streams_poc_lib,
    // HttpClient,
    // HttpClientOptions,
};

use cty;
use std::slice;

/// Create aliases for FFI types for esp32c3, which doesn't have std.
#[cfg(not(feature = "std"))]
mod ffi {
    #![allow(dead_code)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    pub type c_char = u8;
    pub type c_int = i32;
}

// pub mod sys {
//     #![allow(non_upper_case_globals)]
//     #![allow(non_camel_case_types)]
//     #![allow(non_snake_case)]
//     include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
// }

#[repr(C)]
/// Possible errors of the Streams communication stack.
/// The contained values are just for example purposes.
/// The final list will differ a lot.
#[allow(non_camel_case_types)]
pub enum StreamsError {
    STREAMS_OK = 1,
    STREAMS_UNKNOWN_ERROR = -1,
    STREAMS_NODE_NOT_AVAILABLE = -2,
    STREAMS_IOTA_BRIDGE_NOT_AVAILABLE = -3,
}

/// Convert a StreamsError value into a static C string
#[no_mangle]
pub extern "C" fn streams_error_to_string(error: StreamsError) -> *const cty::c_char {
    match error {
        StreamsError::STREAMS_OK=> "STREAMS_OK\0".as_ptr(),
        StreamsError::STREAMS_UNKNOWN_ERROR=> "STREAMS_UNKNOWN_ERROR\0".as_ptr(),
        StreamsError::STREAMS_NODE_NOT_AVAILABLE=> "STREAMS_NODE_NOT_AVAILABLE\0".as_ptr(),
        StreamsError::STREAMS_IOTA_BRIDGE_NOT_AVAILABLE=> "STREAMS_IOTA_BRIDGE_NOT_AVAILABLE\0".as_ptr(),
    }
}

#[repr(C)]
/// Possible errors of the LoRaWAN communication stack.
/// The contained values are just for example purposes.
/// The final list will differ a lot.
#[allow(non_camel_case_types)]
pub enum LoRaWanError {
    LORAWAN_OK = 1,
    LORAWAN_NO_CONNECTION = -1,
}


/// Signature of the callback function allowing the Streams POC library to receive the response for a
/// request that has been send using a send_request_via_lorawan_t function instance.
/// The resolve_request_response_t function will be implemented by the Streams POC library and will be provided to
/// the LoRaWAN communication stack via the response_callback parameter of the send_request_via_lorawan_t function.
/// @param response_data             Binary response data buffer to be returned to the Streams POC library.
///                                  Will be owned by the LoRaWAN communication stack that calls this function.
/// @param length                    Length of response_data
#[allow(non_camel_case_types)]
type resolve_request_response_t = extern fn(response_data: *const cty::uint8_t, length: cty::size_t) -> StreamsError;

/// Signature of the callback function allowing the Streams POC library to send requests via LoRaWAN.
/// This function will be implemented by the LoRaWAN communication stack and will be provided to the Streams POC library
/// via the lorawan_send_callback parameter of the send_message() function.
/// @param request_data              Binary request data buffer to be send via LoRaWAN.
///                                  Will be owned by the Streams POC library code calling this function.
/// @param length                    Length of request_data
/// @param response_callback         Callback function allowing the LoRaWAN communication stack to return response
///                                  data to the Streams POC library.
///                                  These data  have been received via LoRaWAN as a response for the request.
///                                  See resolve_request_response_t help above for more details.
#[allow(non_camel_case_types)]
type send_request_via_lorawan_t = extern fn(request_data: *const cty::uint8_t, length: cty::size_t, response_callback: resolve_request_response_t) -> LoRaWanError;

/// Function provided by the Streams POC library to allow the SUSEE application to send messages encrypted and signed with
/// IOTA Streams via LoRaWan
/// @param message_data              Binary message data to be send
///                                  Will be owned by the SUSEE application code calling this function.
/// @param length                    Length of message_data
/// @param lorawan_send_callback     Callback function allowing the Streams POC library to send requests via LoRaWAN.
///                                  See send_request_via_lorawan_t help above for more details.
#[no_mangle]
pub extern "C" fn send_message(message_data: *const cty::uint8_t, length: cty::size_t, _lorawan_send_callback: send_request_via_lorawan_t) -> StreamsError {
    init_esp_idf_sys_and_logger();
    info!("Starting send_message()");

    assert!(!message_data.is_null());
    match smol::block_on(async {
        debug!("Start smol::block_on");
        let message_slice = unsafe { slice::from_raw_parts(message_data, length) };
        streams_poc_lib::send_message(message_slice).await
    }){
        Ok(_) => {
            StreamsError::STREAMS_OK
        },
        Err(error) => {
            error!("An error occurred while calling process_main(): {}", error);
            StreamsError::STREAMS_UNKNOWN_ERROR
        }
    }
}

#[no_mangle]
pub extern "C" fn process_main() -> i32 {
    init_esp_idf_sys_and_logger();
    info!("Starting process_main()");

    match smol::block_on(async {
        debug!("Start smol::block_on");
        process_main_esp_rs().await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("An error occurred while calling process_main(): {}", error);
        }
    };

    0
}


fn init_esp_idf_sys_and_logger() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    // https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/system/log.html#_CPPv417esp_log_level_setPKc15esp_log_level_t
    // esp_idf_svc::log::EspLogger.set_target_level("*", LevelFilter::Trace);
}