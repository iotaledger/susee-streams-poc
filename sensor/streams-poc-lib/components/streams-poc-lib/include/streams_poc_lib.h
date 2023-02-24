#ifndef streams_poc_lib_h
#define streams_poc_lib_h

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Possible errors of the LoRaWAN communication stack.
 * The contained values are just for example purposes.
 * The final list will differ a lot.
 */
typedef enum LoRaWanError {
  LORAWAN_OK = 1,
  LORAWAN_NO_CONNECTION = -1,
} LoRaWanError;

/**
 * Possible errors of the Streams communication stack.
 * The contained values are just for example purposes.
 * The final list will differ a lot.
 */
typedef enum StreamsError {
  STREAMS_OK = 1,
  STREAMS_UNKNOWN_ERROR = -1,
  STREAMS_INTERNAL_PANIC = -2,
  STREAMS_NODE_NOT_AVAILABLE = -3,
  STREAMS_IOTA_BRIDGE_NOT_AVAILABLE = -4,
  STREAMS_RESPONSE_RESOLVED_WITHOUT_REQUEST = -5,
  STREAMS_RESPONSE_INTERNAL_CHANNEL_ERR = -6,
} StreamsError;

/**
 * Signature of the callback function allowing the Streams POC library to receive the response for a
 * request that has been send using a send_request_via_lorawan_t function instance.
 * The resolve_request_response_t function will be implemented by the Streams POC library and will be provided to
 * the LoRaWAN communication stack via the response_callback parameter of the send_request_via_lorawan_t function.
 * @param response_data             Binary response data buffer to be returned to the Streams POC library.
 *                                  Will be owned by the LoRaWAN communication stack that calls this function.
 * @param length                    Length of response_data
 */
typedef enum StreamsError (*resolve_request_response_t)(const uint8_t *response_data, size_t length);

/**
 * Signature of the callback function allowing the Streams POC library to send requests via LoRaWAN.
 * This function will be implemented by the LoRaWAN communication stack and will be provided to the Streams POC library
 * via the lorawan_send_callback parameter of the send_message() function.
 * @param request_data              Binary request data buffer to be send via LoRaWAN.
 *                                  Will be owned by the Streams POC library code calling this function.
 * @param length                    Length of request_data
 * @param response_callback         Callback function allowing the LoRaWAN communication stack to return response
 *                                  data to the Streams POC library.
 *                                  These data  have been received via LoRaWAN as a response for the request.
 *                                  See resolve_request_response_t help above for more details.
 * @param p_caller_user_data        Pointer to arbitrary data specified by the caller of the send_message()
 *                                  function that resulted in the call of this function.
 *                                  p_caller_user_data can be used by the scope that calls send_message()
 *                                  to communicate with this callback function implementation.
 *
 *                                  If you are using C++ and you have a class that implements the
 *                                  lorawan_send_callback function, containing all logic needed
 *                                  for a send_request_via_lorawan_t implementation, and this class
 *                                  also uses the send_message() function, you may want to
 *                                  set set the p_caller_user_data argument of the send_message() function
 *                                  to the this pointer of your class instance.
 *                                  Here is an Example for a socket connection:
 *
 *                                       class MySocketHandler;
 *
 *                                       LoRaWanError send_request_via_socket(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback, void* p_caller_user_data) {
 *                                          MySocketHandler* p_socket_handler = static_cast<MySocketHandler*>(p_caller_user_data);
 *                                          return p_socket_handler->send_request(request_data, length, response_callback);
 *                                       }
 *
 *                                       class MySocketHandler {
 *                                          ....
 *                                          ....
 *                                          void call_send_message() {
 *                                              send_message(message_data, msg_data_len, send_request_via_socket, NULL, this);     // Here we set p_caller_user_data = this
 *                                          }
 *
 *                                          LoRaWanError send_request(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback) {
 *                                              ....
 *                                          }
 *                                       };
 *
 *                                  Please note that p_caller_user_data is optional and may be NULL in
 *                                  case the caller of the send_message() function specified it to be NULL.
 */
typedef enum LoRaWanError (*send_request_via_lorawan_t)(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback, void *p_caller_user_data);

/**
 * Convert a StreamsError value into a static C string
 */
const char *streams_error_to_string(enum StreamsError error);

/**
 * Function provided by the Streams POC library to allow the SUSEE application to send messages encrypted and signed with
 * IOTA Streams via LoRaWan
 * @param message_data              Binary message data to be send
 *                                  Will be owned by the SUSEE application code calling this function.
 * @param length                    Length of message_data
 * @param lorawan_send_callback     Callback function allowing the Streams POC library to send requests via LoRaWAN.
 *                                  See send_request_via_lorawan_t help above for more details.
 * @param vfs_fat_path              Optional.
 *                                  Path of the directory where the streams channel user state data and
 *                                  other files shall be read/written by the Streams POC library.
 *                                  See function is_streams_channel_initialized() below for further details.
 * @param p_caller_user_data        Optional.
 *                                  Pointer to arbitrary data used by the caller of this function
 *                                  to communicate with the lorawan_send_callback implementation.
 *                                  See send_request_via_lorawan_t help above for more details.
 *                                  If no p_caller_user_data is provided set p_caller_user_data = NULL.
 */
enum StreamsError send_message(const uint8_t *message_data,
                               size_t length,
                               send_request_via_lorawan_t lorawan_send_callback,
                               const char *vfs_fat_path,
                               void *p_caller_user_data);

/**
 * Start an interactive app that can be used to automatically initialize the Streams channel or
 * to query the subscription status of the Streams client.
 * The "sensor_manager" provides the same functionality as the stand alone sensor application
 * contained in the project sensor/main-rust-esp-rs.
 * The sensor can be remote controlled using the 'sensor' app for x86 Linux-PCs
 * (project sensor/main-rust) or the 'management-console' app.
 * For more details about the possible remote commands have a look into the CLI help of those
 * two applications.
 * @param wifi_ssid        Name (Service Set Identifier) of the WiFi to login.
 * @param wifi_pass        Password of the WiFi to login.
 * @param iota_bridge_url  URL of the iota-bridge instance to connect to.
 *                         Example:
 *                            start_sensor_manager("Susee Demo", "susee-rocks", "http://192.168.0.100:50000", NULL);
 * @param vfs_fat_path     Optional.
 *                         Path of the directory where the streams channel user state data and
 *                         other files shall be read/written by the Streams POC library.
 *                         See function is_streams_channel_initialized() below for further details.
 */
int32_t start_sensor_manager(const char *wifi_ssid,
                             const char *wifi_pass,
                             const char *iota_bridge_url,
                             const char *vfs_fat_path);

/**
 * Indicates if this sensor instance has already been initialized.
 * A sensor is initialized if it has subscribed to a streams channel and is ready to send
 * messages via the send_message() function.
 * If this function returns false the initialization process can be started using the
 * function start_sensor_manager(). After start_sensor_manager() has been called you need to run
 * the management-console (project /management console) like this:
 *
 *     $ ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
 *
 * @param vfs_fat_path     Optional.
 *                         Path of the directory where the streams channel user state data and
 *                         other files shall be read/written by the Streams POC library.
 *
 *                         If no FAT filesystem is provided by the caller of this function
 *                         set vfs_fat_path = NULL.
 *
 *                         If a vfs_fat_path value path is defined, a FAT filesystem needs to be
 *                         provided by the caller of this function and following preconditions
 *                         have to be fulfilled:
 *                         * vfs_fat_path must start with the base_path of the vfs_fat data partition
 *                           followed by optional subfolders. The Streams POC library will not
 *                           create any subfolders that are part of vfs_fat_path so all needed
 *                           subfolders must have been created before Streams POC library is used.
 *                         * the FAT filesystem must have been initialized in the SPI flash and
 *                           registered in the VFS e.g. by using esp_vfs_fat_spiflash_mount()
 *                           or equivalent esp-idf functions
 *                           https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-reference/storage/wear-levelling.html
 *
 *                         In case no FAT filesystem is provided resp. vfs_fat_path is set to NULL:
 *                         * the Streams POC library will initialize and use its default
 *                           '/spiflash' data partition.
 *                         * the default '/spiflash' data partition needs to be configured in
 *                           the 'partitions.scv' file of the applications build project.
 *                           See /sensor/streams-poc-lib/partitions.scv as an example.
 *                           https://docs.espressif.com/projects/esp-idf/en/v4.3/esp32/api-guides/partition-tables.html
 *
 *                         Examples:
 *
 *                            // Use the default '/spiflash' partition managed by the Streams POC library
 *                            is_streams_channel_initialized(NULL)
 *
 *                            // Use the root folder of the 'great-spi-flash' partition
 *                            // that has already been initialized using esp_vfs_fat_spiflash_mount()
 *                            // or equivalent esp-idf functions.
 *                            is_streams_channel_initialized("/great-spi-flash")
 *
 *                            // Use the EXISTING subfolder 'streams-folder' in the
 *                            // already initialized data partition 'other-flash-partition'.
 *                            is_streams_channel_initialized("/other-flash-partition/streams-folder")
 */
bool is_streams_channel_initialized(const char *vfs_fat_path);

#endif /* streams_poc_lib_h */
