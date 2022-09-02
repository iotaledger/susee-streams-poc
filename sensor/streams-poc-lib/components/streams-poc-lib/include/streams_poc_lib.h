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
 */
typedef enum LoRaWanError (*send_request_via_lorawan_t)(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback);

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
 */
enum StreamsError send_message(const uint8_t *message_data,
                               size_t length,
                               send_request_via_lorawan_t lorawan_send_callback);

/**
 * Start an interactive app that can be used to automatically initialize the Streams channel or
 * to query the subscription status of the Streams client.
 * The "sensor_manager" provides the same functionality as the stand alone sensor application
 * contained in the project sensor/main-rust-esp-rs.
 * The sensor can be remote controlled using the 'sensor' app for x86 Linux-PCs
 * (project sensor/main-rust) or the 'management-console' app.
 * For more details about the possible remote commands have a look into the CLI help of those
 * two applications.
 */
int32_t start_sensor_manager(void);

/**
 * Indicates if this sensor instance has already been initialized.
 * A sensor is initialized if it has subscribed to a streams channel and is ready to send
 * messages via the send_message() function.
 * If this function returns false the initialization process can be started using the
 * function start_sensor_manager(). After start_sensor_manager() has been called you need to run
 * the management-console (project /management console) like this:
 *
 *     $ ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
 */
bool is_streams_channel_initialized(void);

#endif /* streams_poc_lib_h */
