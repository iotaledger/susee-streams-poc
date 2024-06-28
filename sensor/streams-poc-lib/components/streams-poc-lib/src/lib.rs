use std::{
    slice,
    panic,
    ptr,
    os::raw::c_char,
    ffi::{
        CStr,
        CString,
    },
};

use futures_lite::future;

use cty;

use anyhow::{
    Result,
    bail,
};

use log::*;

use streams_tools::{
    LoraWanRestClientOptions,
    binary_persist::IotaBridgeTcpIpProxySettings
};

use sensor_lib::{
    process_main_esp_rs,
    process_main_esp_rs_lwip,
    streams_poc_lib,
    streams_poc_lib_api_types::{
        StreamsError,
        send_request_via_lorawan_t,
        http_response_call_back_t,
        iota_bridge_tcpip_proxy_options_t,
        streams_client_data_persistence_t,
        streams_client_data_update_call_back_t,
        VfsFatManagement,
        StreamsClientDataStorageType,
        StreamsClientInitializationState,
        VFS_FAT_MOUNT_BASE_PATH,
    },
    esp_rs::{
        LoraWanRestClient,
        client_data_persistence::ClientDataPersistenceOptions
    },
};

static mut IS_ESP_IDF_SYS_AND_LOGGER_INITIALIZED: bool = false;

/// Convert a StreamsError value into a static C string
#[no_mangle]
pub extern "C" fn streams_error_to_string(error: StreamsError) -> *const cty::c_char {
    format!("{}", error).as_ptr()
}

static mut VFS_FAT_MOUNT_BASE_PATH_C_STRING: Option<CString> = None;

/// Returns the base_path that is used to mount the 'storage' data partition if
/// VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED is used.
#[no_mangle]
pub extern "C" fn get_vfs_fat_mount_base_path() -> *const cty::c_char {
    unsafe {
        if VFS_FAT_MOUNT_BASE_PATH_C_STRING.is_none() {
            VFS_FAT_MOUNT_BASE_PATH_C_STRING = Some(CString::new(VFS_FAT_MOUNT_BASE_PATH).unwrap());
        }
        if let Some(base_path) = &VFS_FAT_MOUNT_BASE_PATH_C_STRING {
            base_path.as_ptr() as *const cty::c_char
        } else {
            ptr::null()
        }
    }
}

fn validate_client_data_persistence(p_client_data_persistence: *const streams_client_data_persistence_t) -> bool {
    init_esp_idf_sys_and_logger();
    match unwrap_streams_client_data_persistence(p_client_data_persistence) {
        Ok(scdp_opt) => {
            debug!("[fn validate_client_data_persistence()] unwrap_streams_client_data_persistence() result OK");
            match scdp_opt.validate() {
                Ok(_) => {
                    info!("[fn validate_client_data_persistence()] p_client_data_persistence is OK - returning true");
                    return true;
                },
                Err(err) => {
                    error!("[fn validate_client_data_persistence()] Unwrapped p_client_data_persistence is erroneous: {}", err);
                }
            }
        }
        Err(err) => {
            error!("[fn validate_client_data_persistence()] Error on unwrapping p_client_data_persistence: {}", err);
        }
    }
    false
}


/// Prepare a streams_client_data_persistence_t instance to use the combination of:
/// * StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT
/// * VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED
///
/// Please see the documentation of these constants for further details.
///
/// @param p_client_data_persistence    The streams_client_data_persistence_t instance to be prepared.
/// @return                             true: Success, false: No success
#[no_mangle]
pub extern "C" fn prepare_client_data_storage___vfs_fat___streams_poc_lib_managed(
    p_client_data_persistence: &mut streams_client_data_persistence_t
) -> bool
{
    p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT;
    p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED;
    p_client_data_persistence.vfs_fat_path = ptr::null();
    p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_UNKNOWN;
    p_client_data_persistence.latest_client_data_bytes = ptr::null();
    p_client_data_persistence.latest_client_data_bytes_length = 0;
    p_client_data_persistence.update_client_data_call_back = dummy_streams_client_data_update_call_back_default;
    p_client_data_persistence.p_update_call_back_caller_user_data = ptr::null_mut();
    validate_client_data_persistence(p_client_data_persistence)
}

/// Prepare a streams_client_data_persistence_t instance to use the combination of:
/// * StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT
/// * VfsFatManagement::VFS_FAT_APPLICATION_MANAGED
///
/// Please see the documentation of these constants for further details.
///
/// @param p_client_data_persistence    The streams_client_data_persistence_t instance to be prepared.
/// @param vfs_fat_path                 Path of the directory where files shall be read/written
///                                     by the Streams POC library.
///                                     A FAT filesystem needs to be provided by the caller of this
///                                     function. Please see the documentation of
///                                     VfsFatManagement::VFS_FAT_APPLICATION_MANAGED for the preconditions
///                                     that have to be fulfilled.
/// @return                             true: Success, false: No success
///
/// Examples:
///           // Use the root folder of the 'great-spi-flash' partition
///           // that has already been initialized using esp_vfs_fat_spiflash_mount()
///           // or equivalent esp-idf functions.
///           streams_client_data_persistence_t client_data_persistence;
///           prepare_client_data_storage___vfs_fat___application_managed(
///                 &client_data_persistence,
///                 "/great-spi-flash"
///           );
///
///           // Use the EXISTING subfolder 'streams-folder' in the
///           // already initialized data partition 'other-flash-partition'.
///           streams_client_data_persistence_t client_data_persistence;
///           prepare_client_data_storage___vfs_fat___application_managed(
///                 &client_data_persistence,
///                 "/other-flash-partition/streams-folder"
///           );
#[no_mangle]
#[allow(non_camel_case_types)]
pub extern "C" fn prepare_client_data_storage___vfs_fat___application_managed(
    p_client_data_persistence: &mut streams_client_data_persistence_t,
    vfs_fat_path: *const c_char
) -> bool
{
    p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT;
    p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_APPLICATION_MANAGED;
    p_client_data_persistence.vfs_fat_path = vfs_fat_path;
    p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_UNKNOWN;
    p_client_data_persistence.latest_client_data_bytes = ptr::null();
    p_client_data_persistence.latest_client_data_bytes_length = 0;
    p_client_data_persistence.update_client_data_call_back = dummy_streams_client_data_update_call_back_default;
    p_client_data_persistence.p_update_call_back_caller_user_data = ptr::null_mut();
    validate_client_data_persistence(p_client_data_persistence)
}

/// Prepare a streams_client_data_persistence_t instance to use the combination of:
/// * StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK
/// * VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED
///
/// Please see the documentation of these constants for further details.
///
/// The params of this function are documented at the function
/// prepare_client_data_storage___call_back___application_managed_vfs_fat(), because this function
/// provides these parameters also.
#[no_mangle]
#[allow(non_camel_case_types)]
pub extern "C" fn prepare_client_data_storage___call_back___streams_poc_lib_managed_vfs_fat(
    p_client_data_persistence: &mut streams_client_data_persistence_t,
    client_is_initialized: bool,
    latest_client_data_bytes: *const cty::uint8_t,
    latest_client_data_bytes_length: cty::size_t,
    update_client_data_call_back: streams_client_data_update_call_back_t,
    p_update_call_back_caller_user_data: *mut cty::c_void
) -> bool
{
    if client_is_initialized {
        p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK;
        p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED;
        p_client_data_persistence.vfs_fat_path = ptr::null();
        p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED;
        p_client_data_persistence.latest_client_data_bytes = latest_client_data_bytes;
        p_client_data_persistence.latest_client_data_bytes_length = latest_client_data_bytes_length;
        p_client_data_persistence.update_client_data_call_back = update_client_data_call_back;
        p_client_data_persistence.p_update_call_back_caller_user_data = p_update_call_back_caller_user_data;
    } else {
        p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK;
        p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED;
        p_client_data_persistence.vfs_fat_path = ptr::null();
        p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_NOT_INITIALIZED;
        p_client_data_persistence.latest_client_data_bytes = ptr::null();
        p_client_data_persistence.latest_client_data_bytes_length = 0;
        p_client_data_persistence.update_client_data_call_back = update_client_data_call_back;
        p_client_data_persistence.p_update_call_back_caller_user_data = p_update_call_back_caller_user_data;
    }
    validate_client_data_persistence(p_client_data_persistence)
}

/// Prepare a streams_client_data_persistence_t instance to use the combination of:
/// * StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK
/// * VfsFatManagement::VFS_FAT_APPLICATION_MANAGED
///
/// Please see the documentation of these constants for further details.
///
/// @param p_client_data_persistence    The streams_client_data_persistence_t instance to be prepared.
/// @param vfs_fat_path                 Path of the directory where files shall be read/written
///                                     by the Streams POC library.
///                                     A FAT filesystem needs to be provided by the caller of this
///                                     function. Please see the documentation of
///                                     VfsFatManagement::VFS_FAT_APPLICATION_MANAGED for the preconditions
///                                     that have to be fulfilled. Have a look at the
///                                     prepare_client_data_storage___vfs_fat___application_managed()
///                                     function documentation for some examples.
/// @param client_is_initialized        If the sensor has not been initialized before
///                                     set this to false, otherwise use true.
///                                     In case of client_is_initialized == false
///                                     latest_client_data_bytes and latest_client_data_len
///                                     can be set to NULL resp. 0.
///                                     Otherwise latest_client_data_bytes needs to point
///                                     to the streams client-data array of size
///                                     latest_client_data_len.
/// @param latest_client_data_bytes     Streams client-data that will be used to instantiate
///                                     the streams client.
///                                     In case of client_is_initialized == true, these data
///                                     have been received by the update_client_data_cb when
///                                     a streams_poc_library function has been called once
///                                     before.
/// @param latest_client_data_len       Length of latest_client_data_bytes
/// @param update_client_data_cb        This callback function will be called every time
///                                     when the streams client-data have changed.
///                                     See streams_client_data_update_call_back_t documentation
///                                     for further details.
/// @param p_update_cb_user_data        Optional.
///                                     Will be provided as argument p_caller_user_data
///                                     when update_client_data_cb is called by the streams-poc-lib.
///                                     See function send_request_via_lorawan_t documentation for
///                                     further details.
/// @return                             true: Success, false: No success
#[no_mangle]
#[allow(non_camel_case_types)]
pub extern "C" fn prepare_client_data_storage___call_back___application_managed_vfs_fat(
    p_client_data_persistence: &mut streams_client_data_persistence_t,
    vfs_fat_path: *const c_char,
    client_is_initialized: bool,
    latest_client_data_bytes: *const cty::uint8_t,
    latest_client_data_bytes_length: cty::size_t,
    update_client_data_call_back: streams_client_data_update_call_back_t,
    p_update_call_back_caller_user_data: *mut cty::c_void
) -> bool
{
    if client_is_initialized {
        p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK;
        p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_APPLICATION_MANAGED;
        p_client_data_persistence.vfs_fat_path = vfs_fat_path;
        p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED;
        p_client_data_persistence.latest_client_data_bytes = latest_client_data_bytes;
        p_client_data_persistence.latest_client_data_bytes_length = latest_client_data_bytes_length;
        p_client_data_persistence.update_client_data_call_back = update_client_data_call_back;
        p_client_data_persistence.p_update_call_back_caller_user_data = p_update_call_back_caller_user_data;
    } else {
        p_client_data_persistence.streams_client_data_storage_type = StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK;
        p_client_data_persistence.vfs_fat_management = VfsFatManagement::VFS_FAT_APPLICATION_MANAGED;
        p_client_data_persistence.vfs_fat_path = vfs_fat_path;
        p_client_data_persistence.client_initialization_state = StreamsClientInitializationState::CLIENT_INIT_STATE_NOT_INITIALIZED;
        p_client_data_persistence.latest_client_data_bytes = ptr::null();
        p_client_data_persistence.latest_client_data_bytes_length = 0;
        p_client_data_persistence.update_client_data_call_back = update_client_data_call_back;
        p_client_data_persistence.p_update_call_back_caller_user_data = p_update_call_back_caller_user_data;
    }
    validate_client_data_persistence(p_client_data_persistence)
}

/// Function provided by the Streams POC library to allow the SUSEE application to send messages encrypted and signed with
/// IOTA Streams via LoRaWan
/// @param message_data              Binary message data to be send
///                                  Will be owned by the SUSEE application code calling this function.
/// @param length                    Length of message_data
/// @param lorawan_send_callback     Callback function allowing the Streams POC library to send requests via LoRaWAN.
///                                  See send_request_via_lorawan_t help above for more details.
/// @param p_client_data_persistence Defines how the streams channel client state data and
///                                  other files shall be stored by the Streams POC library.
///                                  Use one of the prepare_.... functions above to create a properly
///                                  initialized p_client_data_persistence instance.
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
    p_client_data_persistence: *const streams_client_data_persistence_t,
    p_caller_user_data: *mut cty::c_void
) -> StreamsError {
    info!("[fn send_message()] Starting");
    init_esp_idf_sys_and_logger();
    info!("[fn send_message()] init_esp_idf_sys_and_logger finished");

    assert!(!message_data.is_null());

    let scdp_options = unwrap_streams_client_data_persistence(p_client_data_persistence)
        .expect("Error on processing the streams client data persistence options p_client_data_persistence");

    let success = panic::catch_unwind(|| -> StreamsError {
        match future::block_on(async {
            debug!("[fn send_message()] Start future::block_on");
            let message_slice = unsafe { slice::from_raw_parts(message_data, length) };
            let ret_val = streams_poc_lib::send_message(
                message_slice,
                lorawan_send_callback,
                scdp_options,
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
///
/// After this function has been called, the sensor can be remote controlled using the
/// x86/PC Sensor Application or the 'management-console' app.
/// For more details about the possible remote commands have a look into the README files of these
/// applications (sensor/README.md and management-console/README.md) and their CLI help.
///
/// The "sensor_manager" repetitively polls commands from the iota-bridge and executes them. To stop
/// the sensor_manager command poll loop please return LoRaWanError::EXIT_SENSOR_MANAGER in your
/// implementation of the lorawan_send_callback.
///
/// Sensor RE-initialization
/// ------------------------
/// A reinitialization can be achieved using this function together with the
/// CLIENT_DATA_STORAGE_CALL_BACK storage type. To perform a reinitialization, provide an empty
/// streams_client_data_persistence_t.latest_client_data_bytes buffer when the
/// 'prepare_client_data_storage___call_back___...' function is called, to prepare the call of this
/// function. As the wallet file, stored in the vfs-fat filesystem is not cleared, the sensor
/// application will be properly reinitialized. See sensor/README.md for more details regarding
/// sensor reinitialization.
/// Sensor reinitialization can also be achieved using CLIENT_DATA_STORAGE_VFS_FAT, if the file
/// used to store the Streams client state is deleted.
///
/// IOTA Bridge Connection
/// ----------------------
/// In general the connection from the Sensor application to the iota-bridge can be realized in one
/// of the following ways:
///
/// * Via LoRaWAN, Bluetooth, a serial wired connection or similar connections that are managed by
///   the Sensor application and using a proxy that transmits the binary packages to the
///   iota-bridge (e.g. an 'Application Server Connector').
///   Here the used iota-bridge is configured in the settings of the proxy.
///   To implement the proxy application the function post_binary_request_to_iota_bridge() can be
///   used to send the payloads to/from the iota-bridge via the "/lorawan-rest/binary_request"
///   REST API endpoint.
/// * Via WiFi, managed by the streams-poc-lib or via an other esp-lwIP based connection.
///   Use function start_sensor_manager_lwip() instead.
///
/// @param send_callback             Callback function allowing the Streams POC library to send
///                                  requests via LoRaWAN, serial wired connections or other
///                                  connection types that are managed by the application.
///                                  See send_request_via_lorawan_t help above for more details.
/// @param dev_eui                   DevEUI of the sensor.
/// @param p_client_data_persistence Defines how the streams channel client state data and
///                                  other files shall be stored by the Streams POC library.
///                                  Use one of the prepare_.... functions above to create a properly
///                                  initialized p_client_data_persistence instance.
/// @param p_caller_user_data        Optional.
///                                  Pointer to arbitrary data used by the caller of this function
///                                  to communicate with the lorawan_send_callback implementation.
///                                  See send_request_via_lorawan_t help above for more details.
///                                  If no p_caller_user_data is provided set p_caller_user_data = NULL.
#[no_mangle]
pub extern "C" fn start_sensor_manager(
    send_callback: send_request_via_lorawan_t,
    dev_eui: *const c_char,
    p_client_data_persistence: *const streams_client_data_persistence_t,
    p_caller_user_data: *mut cty::c_void
) -> i32 {
    init_esp_idf_sys_and_logger();
    info!("[fn start_sensor_manager()] Starting");

    let c_dev_eui: &CStr = unsafe { CStr::from_ptr(dev_eui) };

    let scdp_options = unwrap_streams_client_data_persistence(p_client_data_persistence)
        .expect("Error on processing the streams client data persistence options p_client_data_persistence");

    match future::block_on(async {
        debug!("[fn start_sensor_manager()] Start future::block_on");
        process_main_esp_rs(
            send_callback,
            c_dev_eui.to_str().expect("dev_eui contains invalid utf8 code"),
            scdp_options,
            p_caller_user_data,
        ).await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("[fn start_sensor_manager()] An error occurred while calling process_main(): {}", error);
        }
    };

    libc::EXIT_SUCCESS
}

/// Alternative variant of the start_sensor_manager() function using a streams-poc-lib controlled
/// wifi connection or an other esp-lwIP based connection instead of a 'lorawan_send_callback'.
/// More details regarding esp-lwIP:
/// * https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/lwip.html
/// * Function example_connect()
///   https://github.com/espressif/esp-idf/blob/master/examples/common_components/protocol_examples_common/include/protocol_examples_common.h
///
/// @param iota_bridge_url  URL of the iota-bridge instance to connect to.
///                                 Example:
///                                    start_sensor_manager_wifi("Susee Demo", "susee-rocks", "http://192.168.0.100:50000", NULL);
///
/// @param dev_eui                   DevEUI of the sensor.
/// @param p_client_data_persistence Defines how the streams channel client state data and
///                                  other files shall be stored by the Streams POC library.
///                                  Use one of the prepare_.... functions above to create a properly
///                                  initialized p_client_data_persistence instance.
/// @param wifi_ssid                 Optional.
///                                  Name (Service Set Identifier) of the WiFi to login.
///                                  If wifi_ssid == NULL, the caller of this function has to provide a
///                                  suitable tcp/ip network connection via esp-lwIP.
/// @param wifi_pass                 Optional.
///                                  Password of the WiFi to login.
///                                  Needed if wifi_ssid != NULL otherwise set wifi_pass to NULL.
#[no_mangle]
pub extern "C" fn start_sensor_manager_lwip(
    iota_bridge_url: *const c_char,
    dev_eui: *const c_char,
    p_client_data_persistence: *const streams_client_data_persistence_t,
    wifi_ssid: *const c_char,
    wifi_pass: *const c_char
) -> i32 {
    init_esp_idf_sys_and_logger();
    info!("[fn start_sensor_manager()] Starting");

    let c_iota_bridge_url: &CStr = unsafe { CStr::from_ptr(iota_bridge_url) };
    let c_dev_eui: &CStr = unsafe { CStr::from_ptr(dev_eui) };
    let scdp_options = unwrap_streams_client_data_persistence(p_client_data_persistence)
        .expect("Error on processing the streams client data persistence options p_client_data_persistence");
    let opt_wifi_ssid = get_optional_string_from_c_char_ptr(wifi_ssid, "wifi_ssid")
        .expect("Error on converting optional wifi_ssid to rust String");
    let opt_wifi_pass = get_optional_string_from_c_char_ptr(wifi_pass, "wifi_pass")
        .expect("Error on converting optional wifi_pass to rust String");

    if opt_wifi_ssid.is_some() && opt_wifi_pass.is_none() {
        error!("[fn start_sensor_manager()] wifi_ssid is specified but no wifi_pass has been provided.\
         You always need to provide both wifi_ssid and wifi_pass or set wifi_ssid to NULL");
        return libc::EXIT_SUCCESS;
    }

    match future::block_on(async {
        debug!("[fn start_sensor_manager()] Start future::block_on");
        process_main_esp_rs_lwip(
            c_iota_bridge_url.to_str().expect("iota_bridge_url contains invalid utf8 code"),
            c_dev_eui.to_str().expect("dev_eui contains invalid utf8 code"),
            scdp_options,
            opt_wifi_ssid,
            opt_wifi_pass,
        ).await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("[fn start_sensor_manager()] An error occurred while calling process_main(): {}", error);
        }
    };

    libc::EXIT_SUCCESS
}

/// Indicates if this sensor instance has already been initialized.
/// A sensor is initialized if it has subscribed to a streams channel and is ready to send
/// messages via the send_message() function.
///
/// You only need to use this function in case StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT
/// is used.
///
/// If StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK is used, you don't need
/// this function because in case you've never received and stored a latest streams client
/// data buffer via the streams_client_data_persistence_t.update_client_data_call_back,
/// the sensor has not been initialized before.
/// Otherwise a streams_client_data_persistence_t.client_initialization_state exists and you
/// know, the sensor has been initialized.
///
/// If this function returns false the initialization process can be started using the
/// function start_sensor_manager(). After start_sensor_manager() has been called you need to run
/// the management-console (project /management console) like this:
///
///     $ ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
///
/// @param p_client_data_persistence Defines how the streams channel client state data and
///                                  other files shall be stored by the Streams POC library.
///                                  Use one of the prepare_.... functions above to create a properly
///                                  initialized p_client_data_persistence instance.
#[no_mangle]
pub extern "C" fn is_streams_channel_initialized(p_client_data_persistence: *const streams_client_data_persistence_t) -> bool {
    init_esp_idf_sys_and_logger();
    info!("[fn is_streams_channel_initialized()] Starting");

    let scdp_options = unwrap_streams_client_data_persistence(p_client_data_persistence)
        .expect("Error on processing the streams client data persistence options p_client_data_persistence");

    match future::block_on(async {
        debug!("[fn is_streams_channel_initialized()] Start future::block_on");
        streams_poc_lib::is_streams_channel_initialized(scdp_options).await
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

/// Send a data package to the iota-bridge using the "/lorawan-rest/binary_request" REST API endpoint.
/// This function is NOT used in the Sensor application.
/// This function can be used in a proxy like application (e.g. Application-Server-Connector) that
/// is used to transmit payloads and responses to/from the iota-bridge.
///
/// @param request_data             Binary request data to be send to the iota-bridge.
///                                 These data have been received by the Sensor application
///                                 via the send_callback (parameter of the start_sensor_manager()
///                                 or send_message() function).
///                                 The request data are owned by the proxy application.
/// @param request_length           Length of the request_data
/// @param iota_bridge_proxy_opt    Defines the url of the iota-bridge and the dev_eui of the sensor.
/// @param response_call_back       Used to receive the response data coming from the iota-bridge.
/// @param p_caller_user_data       Optional.
///                                 Pointer to arbitrary data used by the caller of this function
///                                 to communicate with the callers own functions.
///                                 See send_request_via_lorawan_t help above for more details.
///                                 If no p_caller_user_data is provided set p_caller_user_data = NULL.
#[no_mangle]
pub extern "C" fn post_binary_request_to_iota_bridge(
    request_data: *const cty::uint8_t,
    request_length: cty::size_t,
    iota_bridge_proxy_opt: *const iota_bridge_tcpip_proxy_options_t,
    response_call_back: http_response_call_back_t,
    p_caller_user_data: *mut cty::c_void
) {
    // Per definition function pointers in FFI are not nullable so we can not check for NULL pointers here
    assert!(!request_data.is_null());
    assert!(!iota_bridge_proxy_opt.is_null());

    let request_slice = unsafe { slice::from_raw_parts(request_data, request_length) };
    if let Some(proxy_opt) = IotaBridgeTcpIpProxySettings::new_from_iota_bridge_proxy_opt(iota_bridge_proxy_opt) {
        post_lorawan_rest_request(request_slice.to_vec(), proxy_opt, response_call_back,  p_caller_user_data);
    } else {
        error!("[fn post_binary_request_to_iota_bridge()] Undefined or unvalid iota_bridge_proxy_opt");
    }
}

fn post_lorawan_rest_request(
    request_slice: Vec<u8>,
    proxy_opt: IotaBridgeTcpIpProxySettings,
    response_call_back: http_response_call_back_t,
    p_caller_user_data: *mut cty::c_void
) {
    match future::block_on(async {
        let mut lorawan_rest_client = LoraWanRestClient::new(
            Some(
                LoraWanRestClientOptions { iota_bridge_url: proxy_opt.iota_bridge_url.as_str() }
            )
        );
        lorawan_rest_client.post_binary_request_to_iota_bridge(request_slice, proxy_opt.dev_eui.to_string().as_str()).await
    }) {
        Ok(response) => {
            debug!("[fn post_binary_request_to_iota_bridge()] Calling response_call_back with response: {}", response);
            let body_bytes = response.body.as_ptr();
            response_call_back(
                response.status.as_u16(),
                body_bytes,
                response.body.len(),
                p_caller_user_data,
            );
        },
        Err(error) => {
            error!("[fn post_binary_request_to_iota_bridge()] An error occurred while calling streams_tools::post_binary_request_to_iota_bridge(): {}", error);
        }
    };
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

fn unwrap_streams_client_data_persistence(p_client_data_persistence: *const streams_client_data_persistence_t)
                                          -> Result<ClientDataPersistenceOptions> {
    assert!(!p_client_data_persistence.is_null());
    let scdp = unsafe { ptr::read(p_client_data_persistence) };

    let opt_string_vfs_fat_path = unwrap_scdp_vfs_fat(&scdp)?;

    let (latest_client_data, update_call_back) = unwrap_scdp_client_data_storage_type(&scdp)?;

    Ok(ClientDataPersistenceOptions{
        vfs_fat_management: scdp.vfs_fat_management.clone(),
        streams_client_data_storage_type: scdp.streams_client_data_storage_type.clone(),
        vfs_fat_path: opt_string_vfs_fat_path,
        client_initialization_state: scdp.client_initialization_state.clone(),
        latest_client_data_bytes: latest_client_data,
        update_client_data_call_back: update_call_back,
        p_update_call_back_caller_user_data: scdp.p_update_call_back_caller_user_data,
    })
}

fn unwrap_scdp_vfs_fat(scdp: &streams_client_data_persistence_t) -> Result<Option<String>> {
    let opt_string_vfs_fat_path = get_optional_string_from_c_char_ptr(scdp.vfs_fat_path, "vfs_fat_path")
        .expect("Error on converting null terminated C string into utf8 rust String");
    match scdp.vfs_fat_management {
        VfsFatManagement::VFS_FAT_STREAMS_POC_LIB_MANAGED => {
            if let Some(vfs_fat_path) = opt_string_vfs_fat_path {
                bail!("streams_client_data_persistence_t.vfs_fat_path must be NULL if \
                                vfs_fat_management is set to VFS_FAT_STREAMS_POC_LIB_MANAGED. \
                                vfs_fat_path value is {}", vfs_fat_path)
            }
        },
        VfsFatManagement::VFS_FAT_APPLICATION_MANAGED => {
            if opt_string_vfs_fat_path.is_none() {
                bail!("streams_client_data_persistence_t.vfs_fat_path is NULL. \
                                vfs_fat_path must be set to a valid null terminated utf8 string \
                                if vfs_fat_management is set to VFS_FAT_STREAMS_POC_LIB_MANAGED.")
            }
        },
    }
    Ok(opt_string_vfs_fat_path)
}

fn unwrap_scdp_client_data_storage_type(scdp: &streams_client_data_persistence_t)
                        -> Result<(Option<Vec<u8>>, Option<streams_client_data_update_call_back_t>)>
{
    let mut latest_client_data: Option<Vec<u8>> = None;
    let mut update_call_back: Option<streams_client_data_update_call_back_t> = None;
    match scdp.streams_client_data_storage_type {
        StreamsClientDataStorageType::CLIENT_DATA_STORAGE_VFS_FAT => {},
        StreamsClientDataStorageType::CLIENT_DATA_STORAGE_CALL_BACK => {
            latest_client_data = unwrap_latest_client_data(scdp)?;

            // The below ffi check for update_client_data_call_back is not sufficient because it only
            // works if one of the 4 prepare_client_data_storage___... functions is used to create
            // streams_client_data_persistence_t.
            //
            // It is recommended to use Option<streams_client_data_update_call_back_t> in the
            // ffi instead of using pure streams_client_data_update_call_back_t
            // to express function pointers which can be nullable in 'C' but not in Rust.
            //
            // Unfortunately cbindgen generates unusable code if Option<streams_client_data_update_call_back_t>
            // is used in the ffi. Therefore we use a pure streams_client_data_update_call_back_t
            // in the ffi and convert into an Option<streams_client_data_update_call_back_t> here
            // without the ability to check, if the pointer is set to null in the 'C' world.

            if scdp.update_client_data_call_back == dummy_streams_client_data_update_call_back_default {
                bail!("streams_client_data_persistence_t.update_client_data_call_back is set to its. \
                                default value. If streams_client_data_storage_type is set to \
                                CLIENT_DATA_STORAGE_CALL_BACK the update_client_data_call_back needs \
                                to point to a streams_client_data_update_call_back_t function instance.")
            }
            update_call_back = Some(scdp.update_client_data_call_back);
        },
    }

    Ok((latest_client_data, update_call_back))
}

fn unwrap_latest_client_data(scdp: &streams_client_data_persistence_t) -> Result<Option<Vec<u8>>> {
    let mut ret_val: Option<Vec<u8>> = None;
    match &scdp.client_initialization_state {
        StreamsClientInitializationState::CLIENT_INIT_STATE_INITIALIZED => {
            if scdp.latest_client_data_bytes == ptr::null() {
                bail!("streams_client_data_persistence_t.latest_client_data_bytes is NULL. \
                                If streams_client_data_storage_type is set to \
                                CLIENT_DATA_STORAGE_CALL_BACK for an already initialized streams client, \
                                the latest_client_data_bytes pointer must be set to a byte array \
                                containing the latest client-data. \
                                If the streams client has never been initialized the client_initialization_state \
                                must be set to CLIENT_INIT_STATE_NOT_INITIALIZED.")
            }
            let latest_client_data = unsafe {slice::from_raw_parts(
                scdp.latest_client_data_bytes,
                scdp.latest_client_data_bytes_length
            )};
            ret_val = Some(latest_client_data.to_vec());
        },
        StreamsClientInitializationState::CLIENT_INIT_STATE_NOT_INITIALIZED => {},
        _ => {
            bail!("streams_client_data_persistence_t.client_initialization_state value {:?} is not allowed. \
                                    Use either CLIENT_INIT_STATE_INITIALIZED or CLIENT_INIT_STATE_NOT_INITIALIZED.",
                        scdp.client_initialization_state)
        }
    }
    Ok(ret_val)
}

extern fn dummy_streams_client_data_update_call_back_default (
    _client_data_bytes: *const cty::uint8_t,
    _client_data_bytes_length: cty::size_t,
    _p_caller_user_data: *mut cty::c_void
) -> bool
{
    false
}
