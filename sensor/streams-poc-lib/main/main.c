#include <stdio.h>
#include "sdkconfig.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_system.h"
#include "esp_chip_info.h"
// #include "esp_spi_flash.h" // is deprecated, please use spi_flash_mmap.h instead

#include "streams_poc_lib.h"

// This is the binary representation of the content of the file /test/meter_reading_1_compact.json
#define MESSAGE_DATA_LENGTH 213
const uint8_t message_data[MESSAGE_DATA_LENGTH] = {
        0x7b, 0x0a, 0x20, 0x20, 0x22, 0x74, 0x79, 0x70,
        0x65, 0x22, 0x3a, 0x20, 0x22, 0x6d, 0x65, 0x74,
        0x65, 0x72, 0x5f, 0x72, 0x65, 0x61, 0x64, 0x69,
        0x6e, 0x67, 0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22,
        0x72, 0x65, 0x67, 0x69, 0x73, 0x74, 0x65, 0x72,
        0x5f, 0x76, 0x61, 0x6c, 0x75, 0x65, 0x22, 0x3a,
        0x20, 0x32, 0x32, 0x30, 0x31, 0x2e, 0x30, 0x32,
        0x2c, 0x0a, 0x20, 0x20, 0x22, 0x71, 0x75, 0x61,
        0x6c, 0x69, 0x66, 0x69, 0x65, 0x72, 0x22, 0x3a,
        0x20, 0x22, 0x61, 0x2d, 0x70, 0x6c, 0x75, 0x73,
        0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x6f, 0x62,
        0x69, 0x73, 0x5f, 0x76, 0x61, 0x6c, 0x75, 0x65,
        0x22, 0x3a, 0x20, 0x32, 0x32, 0x30, 0x31, 0x2e,
        0x30, 0x32, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x6d,
        0x65, 0x74, 0x65, 0x72, 0x5f, 0x69, 0x64, 0x22,
        0x3a, 0x20, 0x20, 0x20, 0x36, 0x30, 0x32, 0x35,
        0x31, 0x35, 0x30, 0x34, 0x2c, 0x0a, 0x20, 0x20,
        0x22, 0x6d, 0x65, 0x64, 0x69, 0x75, 0x6d, 0x22,
        0x3a, 0x20, 0x22, 0x65, 0x6c, 0x65, 0x63, 0x74,
        0x72, 0x69, 0x63, 0x69, 0x74, 0x79, 0x5f, 0x6b,
        0x77, 0x68, 0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22,
        0x68, 0x65, 0x61, 0x64, 0x65, 0x72, 0x5f, 0x76,
        0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x22, 0x3a,
        0x20, 0x31, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x31,
        0x2d, 0x30, 0x3a, 0x31, 0x2e, 0x38, 0x2e, 0x30,
        0x22, 0x3a, 0x20, 0x20, 0x32, 0x32, 0x30, 0x31,
        0x2e, 0x30, 0x32, 0x0a, 0x7d
};

// Mock data for the LoRaWAN request - requested address could not be found in in the Tangle
#define RESPONSE_NOT_FOUND_IN_TANGLE_LENGTH 6
const uint8_t response_not_found_in_tangle[RESPONSE_NOT_FOUND_IN_TANGLE_LENGTH] = {
        0xFE, 0x01, 0x00, 0x00, 0x00, 0x00
};

// We do not call a socket function in this example code, but if we would do so
// it would like this:
//
//    int err = send(sock, message_data, message_data_length, 0);
//
// Receiving the response would be similar to this:
//
//    int len = recv(sock, rx_buffer, sizeof(rx_buffer) - 1, 0);
//
// To imitate the response data received in rx_buffer we use response_data (see above)
// instead,
//
// A complete example for ESP32 socket client communication can be be found here:
// https://github.com/espressif/esp-idf/tree/v4.2/examples/protocols/sockets/tcp_client

LoRaWanError send_request_via_socket(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback) {
    printf("[streams-poc-lib/main.c - fn send_request_via_socket()] is called with %d bytes of request_data\n", length);

    int i;
    for (i = 0; i < length; i++)
    {
        if (i > 0) printf(":");
        printf("%02X", request_data[i]);
    }
    printf("\n");

    // We act as if we would have called send(sock, ...) followed by recv(sock, ...)
    // as been described above.
    // The next step is to call the streams_poc_lib callback function.
    // We are using response_data array to imitate the rx_buffer we'd received in our
    // recv(sock, ...) call.
    StreamsError err = response_callback(response_not_found_in_tangle, RESPONSE_NOT_FOUND_IN_TANGLE_LENGTH);
    if (err < 0) {
        printf("[streams-poc-lib/main.c - fn send_request_via_socket()] response_callback returned with error code: %s, ", streams_error_to_string(err));
    }

    // As we have not called send(sock, ...) we assume that no LoRaWanError occured.
    return LORAWAN_OK;
}

void app_main(void)
{
    printf("[streams-poc-lib/main.c - fn app_main()] Sensor App is starting!\n");

    /* Print chip information */
    esp_chip_info_t chip_info;
    esp_chip_info(&chip_info);
    printf("This is %s chip with %d CPU cores, WiFi%s%s, ",
            CONFIG_IDF_TARGET,
            chip_info.cores,
            (chip_info.features & CHIP_FEATURE_BT) ? "/BT" : "",
            (chip_info.features & CHIP_FEATURE_BLE) ? "/BLE" : "");

    printf("silicon revision %d, ", chip_info.revision);

//    printf("%dMB %s flash\n", spi_flash_get_chip_size() / (1024 * 1024),
//            (chip_info.features & CHIP_FEATURE_EMB_FLASH) ? "embedded" : "external");

    printf("Free heap: %ld\n", esp_get_free_heap_size());

    if (is_streams_channel_initialized()) {
        printf("[streams-poc-lib/main.c - fn app_main()] Calling send_message for message_data of length %d \n\n", MESSAGE_DATA_LENGTH);
        send_message(message_data, MESSAGE_DATA_LENGTH, send_request_via_socket);
    } else {
        printf("[streams-poc-lib/main.c - fn app_main()] Streams channel for this sensor has not been initialized. Calling start_sensor_manager()");
        start_sensor_manager();
    }
    printf("[streams-poc-lib/main.c - fn app_main()] Exiting Sensor App");
}