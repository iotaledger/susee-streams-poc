use log::*;
use sensor_lib::{
    process_main_esp_rs,
    streams_poc_lib,
    streams_poc_lib::{
        api_types::{
            StreamsError,
            send_request_via_lorawan_t,
        }
    }
    // HttpClient,
    // HttpClientOptions,
};
use futures_lite::future;
use cty;
use std::{
    slice,
    panic,
};

static mut IS_ESP_IDF_SYS_AND_LOGGER_INITIALIZED: bool = false;

/// Convert a StreamsError value into a static C string
#[no_mangle]
pub extern "C" fn streams_error_to_string(error: StreamsError) -> *const cty::c_char {
    format!("{}", error).as_ptr()
}

/// Function provided by the Streams POC library to allow the SUSEE application to send messages encrypted and signed with
/// IOTA Streams via LoRaWan
/// @param message_data              Binary message data to be send
///                                  Will be owned by the SUSEE application code calling this function.
/// @param length                    Length of message_data
/// @param lorawan_send_callback     Callback function allowing the Streams POC library to send requests via LoRaWAN.
///                                  See send_request_via_lorawan_t help above for more details.
#[no_mangle]
pub extern "C" fn send_message(message_data: *const cty::uint8_t, length: cty::size_t, lorawan_send_callback: send_request_via_lorawan_t) -> StreamsError {
    info!("[fn send_message()] Starting");
    init_esp_idf_sys_and_logger();
    info!("[fn send_message()] init_esp_idf_sys_and_logger finished");

    assert!(!message_data.is_null());

    let success = panic::catch_unwind(|| -> StreamsError {
        match future::block_on(async {
            debug!("[fn send_message()] Start future::block_on");
            let message_slice = unsafe { slice::from_raw_parts(message_data, length) };
            let ret_val = streams_poc_lib::send_message(message_slice, lorawan_send_callback).await;
            debug!("[fn send_message()] streams_poc_lib::send_message() ret_val.is_ok() == {}", ret_val.is_ok());
            ret_val
        }){
            Ok(_) => {
                debug!("[fn send_message()] Returning StreamsError::STREAMS_OK");
                StreamsError::STREAMS_OK
            },
            Err(error) => {
                error!("[fn send_message()] An error occurred while calling streams_poc_lib::send_message(): {}", error);
                StreamsError::STREAMS_UNKNOWN_ERROR
            }
        }
    });
    debug!("[fn send_message()] Going to match success");

    let ret_val = match success {
        Ok(streams_error) => { streams_error },
        Err(_error) => {
            error!("[fn send_message()] Function call paniced:");
            StreamsError::STREAMS_INTERNAL_PANIC
        }
    };

    debug!("[fn send_message()] Exciting)");
    ret_val
}

/// Start an interactive app that can be used to automatically initialize the Streams channel or
/// to query the subscription status of the Streams client.
/// The "sensor_manager" provides the same functionality as the stand alone sensor application
/// contained in the project sensor/main-rust-esp-rs.
/// The sensor can be remote controlled using the 'sensor' app for x86 Linux-PCs
/// (project sensor/main-rust) or the 'management-console' app.
/// For more details about the possible remote commands have a look into the CLI help of those
/// two applications.
#[no_mangle]
pub extern "C" fn start_sensor_manager() -> i32 {
    init_esp_idf_sys_and_logger();
    info!("[fn start_sensor_manager()] Starting");

    match future::block_on(async {
        debug!("[fn start_sensor_manager()] Start future::block_on");
        process_main_esp_rs().await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("[fn start_sensor_manager()] An error occurred while calling process_main(): {}", error);
        }
    };

    0
}

/// Indicates if this sensor instance has already been initialized.
/// A sensor is initialized if it has subscribed to a streams channel and is ready to send
/// messages via the send_message() function.
/// If this function returns false the initialization process can be started using the
/// function start_sensor_manager(). After start_sensor_manager() has been called you need to run
/// the management-console (project /management console) like this:
///
///     $ ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
#[no_mangle]
pub extern "C" fn is_streams_channel_initialized() -> bool {
    init_esp_idf_sys_and_logger();
    info!("[fn is_streams_channel_initialized()] Starting");
    match future::block_on(async {
        debug!("[fn is_streams_channel_initialized()] Start future::block_on");
        streams_poc_lib::is_streams_channel_initialized().await
    }){
        Ok(is_initialized) => {
            debug!("[fn is_streams_channel_initialized()] ret_val == {}", is_initialized);
            is_initialized
        },
        Err(error) => {
            error!("[fn is_streams_channel_initialized()] An error occurred while calling streams_poc_lib.is_streams_channel_initialized(): {}", error);
            false
        }
    }
}


fn init_esp_idf_sys_and_logger() {
    let do_initialization;
    unsafe {
        do_initialization = !IS_ESP_IDF_SYS_AND_LOGGER_INITIALIZED;
    }
    if do_initialization {
        // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
        // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
        debug!("[fn init_esp_idf_sys_and_logger()] Starting");
        esp_idf_sys::link_patches();
        debug!("[fn init_esp_idf_sys_and_logger()] link_patches() finished");
        // Bind the log crate to the ESP Logging facilities
        esp_idf_svc::log::EspLogger::initialize_default();
        debug!("[fn init_esp_idf_sys_and_logger()] EspLogger::initialize_default() finished");

        // https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/system/log.html#_CPPv417esp_log_level_setPKc15esp_log_level_t
        // esp_idf_svc::log::EspLogger.set_target_level("*", LevelFilter::Trace);

        unsafe {
            IS_ESP_IDF_SYS_AND_LOGGER_INITIALIZED = true;
        }
    }
}