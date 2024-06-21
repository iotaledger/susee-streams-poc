use std::{
    fmt,
    os::raw::c_char,
};

pub static VFS_FAT_MOUNT_BASE_PATH: &str = "/spiflash";

#[repr(C)]
/// Possible errors while communicating with the IOTA-Tangle via Streams protocol.
/// The contained values are just for example purposes.
/// The final list will differ a lot.
#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug)]
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
    LORAWAN_IOTA_BRIDGE_CONNECTOR_ERROR = -2,
    LORAWAN_EXIT_SENSOR_MANAGER = -100,
}

/// Signature of the callback function allowing the Streams POC library to receive the response for a
/// request that has been send using a send_request_via_lorawan_t function instance.
/// The resolve_request_response_t function will be implemented by the Streams POC library and will be provided to
/// the Sensor application via the response_callback parameter of the send_request_via_lorawan_t function.
/// @param response_data             Binary response data buffer to be returned to the Streams POC library.
///                                  Will be owned by the Sensor application that calls this function.
/// @param length                    Length of response_data
#[allow(non_camel_case_types)]
pub type resolve_request_response_t = extern fn(response_data: *const cty::uint8_t, length: cty::size_t) -> StreamsError;

/// Signature of the callback function allowing the Streams POC library to send requests via LoRaWAN,
/// serial wired connections or other connection types that are managed by the Sensor application.
///
/// This function will be implemented by the Sensor application and will be provided to the Streams POC library
/// via the lorawan_send_callback parameter of the send_message() or start_sensor_manager() functions.
/// @param request_data              Binary request data buffer to be send via LoRaWAN.
///                                  Will be owned by the Streams POC library code calling this function.
/// @param length                    Length of request_data
/// @param response_callback         Callback function allowing the Sensor application to return response
///                                  data to the Streams POC library.
///                                  These data  have been received via LoRaWAN as a response for the request.
///                                  See resolve_request_response_t help above for more details.
/// @param p_caller_user_data        Pointer to arbitrary data specified by the caller of the send_message()
///                                  function that resulted in the call of this function.
///                                  p_caller_user_data can be used by the scope that calls send_message()
///                                  to communicate with this callback function implementation.
///
///                                  If you are using C++ and you have a class that implements the
///                                  lorawan_send_callback function, containing all logic needed
///                                  for a send_request_via_lorawan_t implementation, and this class
///                                  also uses the send_message() function, you may want to
///                                  set set the p_caller_user_data argument of the send_message() function
///                                  to the this pointer of your class instance.
///                                  Here is an Example for a socket connection:
///
///                                       class MySocketHandler;
///
///                                       LoRaWanError send_request_via_socket(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback, void* p_caller_user_data) {
///                                          MySocketHandler* p_socket_handler = static_cast<MySocketHandler*>(p_caller_user_data);
///                                          return p_socket_handler->send_request(request_data, length, response_callback);
///                                       }
///
///                                       class MySocketHandler {
///                                          ....
///                                          ....
///                                          void call_send_message() {
///                                              send_message(message_data, msg_data_len, send_request_via_socket, NULL, this);     // Here we set p_caller_user_data = this
///                                          }
///
///                                          LoRaWanError send_request(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback) {
///                                              ....
///                                          }
///                                       };
///
///                                  Please note that p_caller_user_data is optional and may be NULL in
///                                  case the caller of the send_message() function specified it to be NULL.
#[allow(non_camel_case_types)]
pub type send_request_via_lorawan_t = extern fn(
    request_data: *const cty::uint8_t,
    length: cty::size_t,
    response_callback: resolve_request_response_t,
    p_caller_user_data: *mut cty::c_void
) -> LoRaWanError;

pub use streams_tools::binary_persist::binary_persist_iota_bridge_req::streams_poc_lib_ffi::iota_bridge_tcpip_proxy_options_t;

/// Signature of the callback function used to receive an HTTP-Response.
/// See function post_binary_request_to_iota_bridge() for more details.
/// @param status           HTTP response status.
/// @param body_bytes       Binary data of the response body.
///                         The data are owned by the streams_poc_library.
/// @param body_length      Length of the body_bytes
#[allow(non_camel_case_types)]
pub type http_response_call_back_t = extern fn(
    status: u16,
    body_bytes: *const cty::uint8_t,
    body_length: cty::size_t,
    p_caller_user_data: *mut cty::c_void
);

/// Defines how vfs_fat data partitions, needed to store files in spiflash, shall be managed
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone, Debug)]
pub enum VfsFatManagement {
    /// The Streams POC library will initialize and use its default
    /// '/spiflash' data partition.
    /// To use this option, the default 'storage' data partition
    /// needs to be configured in the 'partitions.scv' file of the
    /// applications build project.
    /// See /sensor/streams-poc-lib/partitions.scv as an example.
    /// https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-guides/partition-tables.html
    VFS_FAT_STREAMS_POC_LIB_MANAGED = 1,

    /// The Sensor application using the streams-poc-lib functions
    /// is responsible to manage a vfs_fat data partition.
    ///
    /// Following preconditions have to be fulfilled:
    /// * streams_client_data_persistence_t.vfs_fat_path must start with the base_path
    ///   of the vfs_fat data partition followed by optional subfolders.
    ///   The Streams POC library will not create any subfolders that are part
    ///   of vfs_fat_path so all needed subfolders must have been created before the
    ///   Streams POC library is used.
    /// * the FAT filesystem must have been initialized in the SPI flash and
    ///   registered in the VFS e.g. by using esp_vfs_fat_spiflash_mount()
    ///   or equivalent esp-idf functions
    ///   https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-reference/storage/wear-levelling.html
    VFS_FAT_APPLICATION_MANAGED = 2,
}

/// Defines how streams client data shall be stored
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone, Debug)]
pub enum StreamsClientDataStorageType {
    /// Streams client data are stored on the in the vfs_fat data partition
    /// managed by the Streams POC library or by the Sensor Application,
    /// according to the used VfsFatManagement option.
    ///
    /// Use one of the following functions to create a properly initialized
    /// streams_client_data_persistence_t instance:
    /// * prepare_client_data_storage___vfs_fat___streams_poc_lib_managed()
    /// * prepare_client_data_storage___vfs_fat___application_managed()
    CLIENT_DATA_STORAGE_VFS_FAT = 1,

    /// Storage of the streams client data is fully managed by the
    /// Sensor application:
    /// * initial streams client data are provided by the application
    ///   via an initial data buffer:
    ///   streams_client_data_persistence_t.latest_client_data_bytes
    /// * after the streams client data have changed the resulting
    ///   latest data are handed to the application via a callback
    ///   function that is called by the streams-poc-lib:
    ///   streams_client_data_persistence_t.update_client_data_call_back
    ///
    /// Use one of the following functions to create a properly initialized
    /// streams_client_data_persistence_t instance:
    /// * prepare_client_data_storage___call_back___streams_poc_lib_managed_vfs_fat()
    /// * prepare_client_data_storage___call_back___application_managed_vfs_fat()
    CLIENT_DATA_STORAGE_CALL_BACK = 2,
}

/// Indicates the initialization state of the sensor
#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(PartialEq, Clone, Debug)]
pub enum StreamsClientInitializationState {
    CLIENT_INIT_STATE_UNKNOWN = 0,
    CLIENT_INIT_STATE_NOT_INITIALIZED = 1,
    CLIENT_INIT_STATE_INITIALIZED = 2,
}


/// Signature of the callback function used to receive latest Streams client state data.
/// See following function for more details:
/// * prepare_client_data_storage___call_back___streams_poc_lib_managed_vfs_fat()
/// * prepare_client_data_storage___call_back___application_managed_vfs_fat()
///
/// The callback function:
/// * must be implemented by the caller of the above listed functions.
/// * will be called by the streams_poc_library to provide the client state data.
/// * must store the data so that the data can be provided as
///   streams_client_data_persistence_t.latest_client_data_bytes when a streams_poc_library
///   function is called next time.
///
///   *************************************************************************************
///   * ATTENTION:                                                                        *
///   *            In case the callback function is not able to store the client state    *
///   *            data, the callback must return false.                                  *
///   *            In the current implementation of the susee-streams-poc applications    *
///   *            this would result in a streams-channel that ends with the latest send  *
///   *            message. The reason for this is, that the Sensor would send its next   *
///   *            message based on an older channel state as the state that is used by   *
///   *            all other channel participants, because these participants have        *
///   *            received the latest message but do not know that the latest state has  *
///   *            been lost.                                                             *
///   *                                                                                   *
///   *            -------------------------------------------------------------------    *
///   *            - Make sure that the client state data are always properly stored -    *
///   *            - and the callback function never needs to return false.          -    *
///   *            -------------------------------------------------------------------    *
///   *                                                                                   *
///   *           In future versions of the susee-streams-poc applications, a command     *
///   *           could be implemented to make other streams channel participants         *
///   *           skip the latest state and proceed with the previous state.              *
///   *                                                                                   *
///   *************************************************************************************
///
/// @param client_data_bytes        Binary data of the response body.
///                                 The data are owned by the streams_poc_library.
/// @param client_data_bytes_length Length of the client_data_bytes
/// @param p_caller_user_data       Pointer to arbitrary data specified by the caller of the above
///                                 listed functions. p_caller_user_data can be used by the scope
///                                 that calls streams_poc_library functions to communicate with
///                                 the callback function implementation.
///                                 Have a look at function send_request_via_lorawan_t documentation
///                                 for an example and more details.
/// @return                         true:   Latest Streams client state data have been successfully
///                                         processed.
///                                 false:  The client state data could not be stored so that the
///                                         client state remains in the previous state
///                                         (that results from the last successful callback function
///                                         call). See ATTENTION-Box above for more details.
#[allow(non_camel_case_types)]
pub type streams_client_data_update_call_back_t = extern fn(
    client_data_bytes: *const cty::uint8_t,
    client_data_bytes_length: cty::size_t,
    p_caller_user_data: *mut cty::c_void
) -> bool;

/// This struct bundles all data needed to manage streams-client-data-persistence.
///
/// Do not set the values of this struct yourself. Instead, use one of the following functions
/// to prepare a streams_client_data_persistence_t instance :
///
///     * prepare_client_data_storage___vfs_fat___streams_poc_lib_managed()
///     * prepare_client_data_storage___vfs_fat___application_managed()
///     * prepare_client_data_storage___call_back___streams_poc_lib_managed_vfs_fat()
///     * prepare_client_data_storage___call_back___application_managed_vfs_fat()
///
/// Please have a look at these functions for more details.
///
/// Usage example:
///
///     streams_client_data_persistence_t client_data_persistence;
///     bool success = prepare_client_data_storage___vfs_fat___application_managed(
///         &client_data_persistence,
///         "/awesome-data"
///     );
///     ...
///     send_message(MESSAGE_DATA, MESSAGE_DATA_LENGTH, lorawan_send_cb_fun, &client_data_persistence, &lorawan_send_cb_user_data);
///
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct streams_client_data_persistence_t {
    pub vfs_fat_management: VfsFatManagement,
    pub streams_client_data_storage_type: StreamsClientDataStorageType,
    pub vfs_fat_path: *const c_char,
    pub client_initialization_state: StreamsClientInitializationState,
    pub latest_client_data_bytes: *const cty::uint8_t,
    pub latest_client_data_bytes_length: cty::size_t,
    pub update_client_data_call_back: streams_client_data_update_call_back_t,
    pub p_update_call_back_caller_user_data: *mut cty::c_void
}