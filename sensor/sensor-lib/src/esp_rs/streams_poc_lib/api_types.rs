use std::fmt;

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
    STREAMS_INTERNAL_PANIC = -2,
    STREAMS_NODE_NOT_AVAILABLE = -3,
    STREAMS_IOTA_BRIDGE_NOT_AVAILABLE = -4,
    STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST = -5,
    STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR = -6,
}

impl fmt::Display for StreamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let streams_err = match self {
            StreamsError::STREAMS_OK => "STREAMS_OK",
            StreamsError::STREAMS_INTERNAL_PANIC => "STREAMS_INTERNAL_PANIC",
            StreamsError::STREAMS_NODE_NOT_AVAILABLE => "STREAMS_NODE_NOT_AVAILABLE",
            StreamsError::STREAMS_IOTA_BRIDGE_NOT_AVAILABLE => "STREAMS_IOTA_BRIDGE_NOT_AVAILABLE",
            StreamsError::STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST => "STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST",
            StreamsError::STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR => "STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR",
            _ => "STREAMS_UNKNOWN_ERROR",
        };
        write!(f, "{}", streams_err)
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
pub type resolve_request_response_t = extern fn(response_data: *const cty::uint8_t, length: cty::size_t) -> StreamsError;

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
pub type send_request_via_lorawan_t = extern fn(request_data: *const cty::uint8_t, length: cty::size_t, response_callback: resolve_request_response_t) -> LoRaWanError;

