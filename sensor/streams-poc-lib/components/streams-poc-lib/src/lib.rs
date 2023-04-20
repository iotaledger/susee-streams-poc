use log::*;
use sensor_lib::{
    process_main_esp_rs,
    process_main_esp_rs_wifi,
    streams_poc_lib,
    streams_poc_lib_api_types::{
        StreamsError,
        send_request_via_lorawan_t,
    }
    // HttpClient,
    // HttpClientOptions,
};

use std::{
    slice,
    panic,
    ptr,
    os::raw::c_char,
    ffi::{
        CStr
    },
};

use futures_lite::future;

use cty;

use anyhow::{
    Result,
    bail,
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
/// @param vfs_fat_path              Optional.
///                                  Path of the directory where the streams channel user state data and
///                                  other files shall be read/written by the Streams POC library.
///                                  See function is_streams_channel_initialized() below for further details.
/// @param p_caller_user_data        Optional.
///                                  Pointer to arbitrary data used by the caller of this function
///                                  to communicate with the lorawan_send_callback implementation.
///                                  See send_request_via_lorawan_t help above for more details.
///                                  If no p_caller_user_data is provided set p_caller_user_data = NULL.
#[no_mangle]
pub extern "C" fn send_message(
    message_data: *const cty::uint8_t,
    length: cty::size_t,
    lorawan_send_callback: send_request_via_lorawan_t,
    vfs_fat_path: *const c_char,
    p_caller_user_data: *mut cty::c_void
) -> StreamsError {
    info!("[fn send_message()] Starting");
    init_esp_idf_sys_and_logger();
    info!("[fn send_message()] init_esp_idf_sys_and_logger finished");

    assert!(!message_data.is_null());

    let opt_string_vfs_fat_path = get_optional_string_from_c_char_ptr(vfs_fat_path, "vfs_fat_path")
        .expect("Error on converting null terminated C string into utf8 rust String");

    let success = panic::catch_unwind(|| -> StreamsError {
        match future::block_on(async {
            debug!("[fn send_message()] Start future::block_on");
            let message_slice = unsafe { slice::from_raw_parts(message_data, length) };
            let ret_val = streams_poc_lib::send_message(
                message_slice,
                lorawan_send_callback,
                opt_string_vfs_fat_path,
                p_caller_user_data,
            ).await;
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
///
/// The "sensor_manager" repetitively polls commands from the IOTA-Bridge and executes them. To stop
/// the sensor_manager command poll loop please return LoRaWanError::EXIT_SENSOR_MANAGER in your
/// implementation of the lorawan_send_callback.
///
/// @param lorawan_send_callback    Callback function allowing the Streams POC library to send requests via LoRaWAN.
///                                 See send_request_via_lorawan_t help above for more details.
/// @param vfs_fat_path             Optional.
///                                 Path of the directory where the streams channel user state data and
///                                 other files shall be read/written by the Streams POC library.
///                                 See function is_streams_channel_initialized() below for further details.
/// @param p_caller_user_data       Optional.
///                                 Pointer to arbitrary data used by the caller of this function
///                                 to communicate with the lorawan_send_callback implementation.
///                                 See send_request_via_lorawan_t help above for more details.
///                                 If no p_caller_user_data is provided set p_caller_user_data = NULL.
#[no_mangle]
pub extern "C" fn start_sensor_manager(
    lorawan_send_callback: send_request_via_lorawan_t,
    vfs_fat_path: *const c_char,
    p_caller_user_data: *mut cty::c_void
) -> i32 {
    init_esp_idf_sys_and_logger();
    info!("[fn start_sensor_manager()] Starting");

    let opt_vfs_fat_path = get_optional_string_from_c_char_ptr(vfs_fat_path, "vfs_fat_path")
        .expect("Error on converting null terminated C string into utf8 rust String");

    match future::block_on(async {
        debug!("[fn start_sensor_manager()] Start future::block_on");
        process_main_esp_rs(
            lorawan_send_callback,
            p_caller_user_data,
            opt_vfs_fat_path
        ).await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("[fn start_sensor_manager()] An error occurred while calling process_main(): {}", error);
        }
    };

    0
}

/// Alternative variant of the start_sensor_manager() function using a streams-poc-lib controlled
/// wifi connection instead of a 'lorawan_send_callback'.
///
/// @param wifi_ssid        Name (Service Set Identifier) of the WiFi to login.
/// @param wifi_pass        Password of the WiFi to login.
/// @param iota_bridge_url  Same as start_sensor_manager() iota_bridge_url parameter.
/// @param vfs_fat_path     Optional.
///                         Same as start_sensor_manager() vfs_fat_path parameter.
#[no_mangle]
pub extern "C" fn start_sensor_manager_wifi(
    wifi_ssid: *const c_char,
    wifi_pass: *const c_char,
    iota_bridge_url: *const c_char,
    vfs_fat_path: *const c_char
) -> i32 {
    init_esp_idf_sys_and_logger();
    info!("[fn start_sensor_manager()] Starting");

    let c_wifi_ssid: &CStr = unsafe { CStr::from_ptr(wifi_ssid) };
    let c_wifi_pass: &CStr = unsafe { CStr::from_ptr(wifi_pass) };
    let c_iota_bridge_url: &CStr = unsafe { CStr::from_ptr(iota_bridge_url) };
    let opt_vfs_fat_path = get_optional_string_from_c_char_ptr(vfs_fat_path, "vfs_fat_path")
        .expect("Error on converting null terminated C string into utf8 rust String");

    match future::block_on(async {
        debug!("[fn start_sensor_manager()] Start future::block_on");
        process_main_esp_rs_wifi(
            c_wifi_ssid.to_str().expect("wifi_ssid contains invalid utf8 code"),
            c_wifi_pass.to_str().expect("wifi_pass contains invalid utf8 code"),
            c_iota_bridge_url.to_str().expect("iota_bridge_url contains invalid utf8 code"),
            opt_vfs_fat_path
        ).await
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
///
/// @param vfs_fat_path     Optional.
///                         Path of the directory where the streams channel user state data and
///                         other files shall be read/written by the Streams POC library.
///
///                         If no FAT filesystem is provided by the caller of this function
///                         set vfs_fat_path = NULL.
///
///                         If a vfs_fat_path value path is defined, a FAT filesystem needs to be
///                         provided by the caller of this function and following preconditions
///                         have to be fulfilled:
///                         * vfs_fat_path must start with the base_path of the vfs_fat data partition
///                           followed by optional subfolders. The Streams POC library will not
///                           create any subfolders that are part of vfs_fat_path so all needed
///                           subfolders must have been created before Streams POC library is used.
///                         * the FAT filesystem must have been initialized in the SPI flash and
///                           registered in the VFS e.g. by using esp_vfs_fat_spiflash_mount()
///                           or equivalent esp-idf functions
///                           https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-reference/storage/wear-levelling.html
///
///                         In case no FAT filesystem is provided resp. vfs_fat_path is set to NULL:
///                         * the Streams POC library will initialize and use its default
///                           '/spiflash' data partition.
///                         * the default '/spiflash' data partition needs to be configured in
///                           the 'partitions.scv' file of the applications build project.
///                           See /sensor/streams-poc-lib/partitions.scv as an example.
///                           https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-guides/partition-tables.html
///
///                         Examples:
///
///                            // Use the default '/spiflash' partition managed by the Streams POC library
///                            is_streams_channel_initialized(NULL)
///
///                            // Use the root folder of the 'great-spi-flash' partition
///                            // that has already been initialized using esp_vfs_fat_spiflash_mount()
///                            // or equivalent esp-idf functions.
///                            is_streams_channel_initialized("/great-spi-flash")
///
///                            // Use the EXISTING subfolder 'streams-folder' in the
///                            // already initialized data partition 'other-flash-partition'.
///                            is_streams_channel_initialized("/other-flash-partition/streams-folder")
#[no_mangle]
pub extern "C" fn is_streams_channel_initialized(vfs_fat_path: *const c_char) -> bool {
    init_esp_idf_sys_and_logger();
    info!("[fn is_streams_channel_initialized()] Starting");

    let opt_string_vfs_fat_path = get_optional_string_from_c_char_ptr(vfs_fat_path, "vfs_fat_path")
        .expect("Error on converting null terminated C string into utf8 rust String");

    match future::block_on(async {
        debug!("[fn is_streams_channel_initialized()] Start future::block_on");
        streams_poc_lib::is_streams_channel_initialized(opt_string_vfs_fat_path).await
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

fn get_optional_string_from_c_char_ptr<'a>(char_ptr_in: *const c_char, argument_name: &str) -> Result<Option<String>> {
    if char_ptr_in != ptr::null() {
        let cstr_in: &CStr = unsafe { CStr::from_ptr(char_ptr_in) };
        match cstr_in.to_str() {
            Ok(utf8_str_in) => {
                Ok(Some(String::from(utf8_str_in)))
            }
            Err(e) => {
                error!("The specified argument '{}' contains invalid utf8 code. Error: {}", argument_name, e);
                bail!("The specified argument '{}' contains invalid utf8 code. Error: {}", argument_name, e);
            }
        }
    } else {
        Ok(None)
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