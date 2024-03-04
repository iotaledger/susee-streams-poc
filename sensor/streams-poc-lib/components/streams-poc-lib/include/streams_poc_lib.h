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
  LORAWAN_IOTA_BRIDGE_CONNECTOR_ERROR = -2,
  LORAWAN_EXIT_SENSOR_MANAGER = -100,
} LoRaWanError;

/**
 * Possible errors while communicating with the IOTA-Tangle via Streams protocol.
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
 * the Sensor application via the response_callback parameter of the send_request_via_lorawan_t function.
 * @param response_data             Binary response data buffer to be returned to the Streams POC library.
 *                                  Will be owned by the Sensor application that calls this function.
 * @param length                    Length of response_data
 */
typedef enum StreamsError (*resolve_request_response_t)(const uint8_t *response_data, size_t length);

/**
 * Signature of the callback function allowing the Streams POC library to send requests via LoRaWAN,
 * serial wired connections or other connection types that are managed by the Sensor application.
 *
 * This function will be implemented by the Sensor application and will be provided to the Streams POC library
 * via the lorawan_send_callback parameter of the send_message() or start_sensor_manager() functions.
 * @param request_data              Binary request data buffer to be send via LoRaWAN.
 *                                  Will be owned by the Streams POC library code calling this function.
 * @param length                    Length of request_data
 * @param response_callback         Callback function allowing the Sensor application to return response
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
 * Used with post_binary_request_to_iota_bridge() function
 * @param dev_eui              DevEUI of the sensor used by the IOTA-Bridge to identify the sensor.
 * @param iota_bridge_url      URL of the iota-bridge instance to connect to.
 *                                 Example: "http://192.168.0.100:50000"
 */
typedef struct iota_bridge_tcpip_proxy_options_t {
  const char *dev_eui;
  const char *iota_bridge_url;
} iota_bridge_tcpip_proxy_options_t;

/**
 * Signature of the callback function used to receive an HTTP-Response.
 * See function post_binary_request_to_iota_bridge() for more details.
 * @param status           HTTP response status.
 * @param body_bytes       Binary data of the response body.
 *                         The data sre owned by the streams_poc_library.
 * @param body_length      Length of the body_bytes
 */
typedef void (*http_response_call_back_t)(uint16_t status, const uint8_t *body_bytes, size_t body_length, void *p_caller_user_data);

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
 *
 * The "sensor_manager" repetitively polls commands from the iota-bridge and executes them. To stop
 * the sensor_manager command poll loop please return LoRaWanError::EXIT_SENSOR_MANAGER in your
 * implementation of the lorawan_send_callback.
 *
 * In general the connection from the Sensor application to the iota-bridge can be realized in one
 * of the following ways:
 *
 * * Via LoRaWAN, Bluetooth, a serial wired connection or similar connections that are managed by
 *   the Sensor application and using a proxy that transmits the binary packages to the
 *   iota-bridge (e.g. an 'Application Server Connector').
 *   Here the used iota-bridge is configured in the settings of the proxy.
 *   To implement the proxy application the function post_binary_request_to_iota_bridge() can be
 *   used to send the payloads to/from the iota-bridge via the "/lorawan-rest/binary_request"
 *   REST API endpoint.
 * * Via WiFi, managed by the streams-poc-lib or via an other esp-lwIP based connection.
 *   Use function start_sensor_manager_lwip() instead.
 *
 * @param send_callback            Callback function allowing the Streams POC library to send
 *                                 requests via LoRaWAN, serial wired connections or other
 *                                 connection types that are managed by the application.
 *                                 See send_request_via_lorawan_t help above for more details.
 * @param dev_eui                  DevEUI of the sensor.
 * @param vfs_fat_path             Optional.
 *                                 Path of the directory where the streams channel user state data and
 *                                 other files shall be read/written by the Streams POC library.
 *                                 See function is_streams_channel_initialized() below for further details.
 * @param p_caller_user_data       Optional.
 *                                 Pointer to arbitrary data used by the caller of this function
 *                                 to communicate with the lorawan_send_callback implementation.
 *                                 See send_request_via_lorawan_t help above for more details.
 *                                 If no p_caller_user_data is provided set p_caller_user_data = NULL.
 */
int32_t start_sensor_manager(send_request_via_lorawan_t send_callback,
                             const char *dev_eui,
                             const char *vfs_fat_path,
                             void *p_caller_user_data);

/**
 * Alternative variant of the start_sensor_manager() function using a streams-poc-lib controlled
 * wifi connection or an other esp-lwIP based connection instead of a 'lorawan_send_callback'.
 * More details regarding esp-lwIP:
 * * https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/api-guides/lwip.html
 * * Function example_connect()
 *   https://github.com/espressif/esp-idf/blob/master/examples/common_components/protocol_examples_common/include/protocol_examples_common.h
 *
 * @param iota_bridge_url  URL of the iota-bridge instance to connect to.
 *                                 Example:
 *                                    start_sensor_manager_wifi("Susee Demo", "susee-rocks", "http://192.168.0.100:50000", NULL);
 * @param dev_eui          DevEUI of the sensor.
 * @param vfs_fat_path     Optional.
 *                         Same as start_sensor_manager() vfs_fat_path parameter.
 * @param wifi_ssid        Optional.
 *                         Name (Service Set Identifier) of the WiFi to login.
 *                         If wifi_ssid == NULL, the caller of this function has to provide a
 *                         suitable tcp/ip network connection via esp-lwIP.
 * @param wifi_pass        Optional.
 *                         Password of the WiFi to login.
 *                         Needed if wifi_ssid != NULL otherwise set wifi_pass to NULL.
 */
int32_t start_sensor_manager_lwip(const char *iota_bridge_url,
                                  const char *dev_eui,
                                  const char *vfs_fat_path,
                                  const char *wifi_ssid,
                                  const char *wifi_pass);

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

/**
 * Send a data package to the iota-bridge using the "/lorawan-rest/binary_request" REST API endpoint.
 * This function is NOT used in the Sensor application.
 * This function can be used in a proxy like application (e.g. Application-Server-Connector) that
 * is used to transmit payloads and responses to/from the iota-bridge.
 *
 * @param request_data             Binary request data to be send to the iota-bridge.
 *                                 These data have been received by the Sensor application
 *                                 via the send_callback (parameter of the start_sensor_manager()
 *                                 or send_message() function).
 *                                 The request data are owned by the proxy application.
 * @param request_length           Length of the request_data
 * @param iota_bridge_proxy_opt    Defines the url of the iota-bridge and the dev_eui of the sensor.
 * @param response_call_back       Used to receive the response data coming from the iota-bridge.
 * @param p_caller_user_data       Optional.
 *                                 Pointer to arbitrary data used by the caller of this function
 *                                 to communicate with the callers own functions.
 *                                 See send_request_via_lorawan_t help above for more details.
 *                                 If no p_caller_user_data is provided set p_caller_user_data = NULL.
 */
void post_binary_request_to_iota_bridge(const uint8_t *request_data,
                                        size_t request_length,
                                        const struct iota_bridge_tcpip_proxy_options_t *iota_bridge_proxy_opt,
                                        http_response_call_back_t response_call_back,
                                        void *p_caller_user_data);

#endif /* streams_poc_lib_h */
