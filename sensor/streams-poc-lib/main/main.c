#include <stdio.h>
#include <string.h>
#include "sdkconfig.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"
#include "esp_system.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_mac.h"
#include "nvs_flash.h"

#include "lwip/err.h"
#include "lwip/sys.h"
#include "lwip/sockets.h"

#include <http_parser.h>

#include "esp_chip_info.h"
// #include "esp_spi_flash.h" // is deprecated, please use spi_flash_mmap.h instead

#include "streams_poc_lib.h"

#include <inttypes.h>

/* ########################################################################################
   ############################ Test CONFIG ###############################################
   ######################################################################################## */

/* This test application uses several settings that are defined as environment variables
   at compile time:
   * SENSOR_MAIN_POC_WIFI_SSID
   * SENSOR_MAIN_POC_WIFI_PASS
   * SENSOR_MAIN_POC_TANGLE_PROXY_URL

   Have a look at the main README.md file of this repository for an example how
   to define these variables.
*/

#define ESP_WIFI_SCAN_AUTH_MODE_THRESHOLD WIFI_AUTH_WPA2_PSK
#define STREAMS_POC_LIB_TEST_MAXIMUM_RETRY 5
#define SEND_BUFFER_SIZE 4096

/*Comment one of the following flags according to your SENSOR_MAIN_POC_TANGLE_PROXY_URL.
  BTW: The value of the flag is not of importance.*/
#define CONFIG_EXAMPLE_IPV4 true
// #define CONFIG_EXAMPLE_IPV6 true

/* ########################################################################################
   ############################ END of test CONFIG ########################################
   ######################################################################################## */


#ifdef CONFIG_EXAMPLE_IPV4
    typedef struct sockaddr_in dest_addr_t;
#elif defined CONFIG_EXAMPLE_IPV6
    typedef struct sockaddr_in6 dest_addr_t;
#endif

/* FreeRTOS event group to signal when we are connected*/
static EventGroupHandle_t s_wifi_event_group;

/* The event group allows multiple bits for each event, but we only care about two events:
 * - we are connected to the AP with an IP
 * - we failed to connect after the maximum amount of retries */
#define WIFI_CONNECTED_BIT BIT0
#define WIFI_FAIL_BIT      BIT1

static const char *TAG = "wifi station";

static int s_retry_num = 0;

static int s_socket_handle;

// This is the binary representation of the content of the file /test/meter_reading_1.json
#define MESSAGE_DATA_LENGTH 80 // 213
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
//        0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x6f, 0x62,
//        0x69, 0x73, 0x5f, 0x76, 0x61, 0x6c, 0x75, 0x65,
//        0x22, 0x3a, 0x20, 0x32, 0x32, 0x30, 0x31, 0x2e,
//        0x30, 0x32, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x6d,
//        0x65, 0x74, 0x65, 0x72, 0x5f, 0x69, 0x64, 0x22,
//        0x3a, 0x20, 0x20, 0x20, 0x36, 0x30, 0x32, 0x35,
//        0x31, 0x35, 0x30, 0x34, 0x2c, 0x0a, 0x20, 0x20,
//        0x22, 0x6d, 0x65, 0x64, 0x69, 0x75, 0x6d, 0x22,
//        0x3a, 0x20, 0x22, 0x65, 0x6c, 0x65, 0x63, 0x74,
//        0x72, 0x69, 0x63, 0x69, 0x74, 0x79, 0x5f, 0x6b,
//        0x77, 0x68, 0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22,
//        0x68, 0x65, 0x61, 0x64, 0x65, 0x72, 0x5f, 0x76,
//        0x65, 0x72, 0x73, 0x69, 0x6f, 0x6e, 0x22, 0x3a,
//        0x20, 0x31, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x31,
//        0x2d, 0x30, 0x3a, 0x31, 0x2e, 0x38, 0x2e, 0x30,
//        0x22, 0x3a, 0x20, 0x20, 0x32, 0x32, 0x30, 0x31,
//        0x2e, 0x30, 0x32, 0x0a, 0x7d
};

static void wifi_init_event_handler(void* arg, esp_event_base_t event_base,
                                int32_t event_id, void* event_data)
{
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        if (s_retry_num < STREAMS_POC_LIB_TEST_MAXIMUM_RETRY) {
            esp_wifi_connect();
            s_retry_num++;
            ESP_LOGI(TAG, "retry to connect to the AP");
        } else {
            xEventGroupSetBits(s_wifi_event_group, WIFI_FAIL_BIT);
        }
        ESP_LOGI(TAG,"connect to the AP fail");
    } else if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t* event = (ip_event_got_ip_t*) event_data;
        ESP_LOGI(TAG, "got ip:" IPSTR, IP2STR(&event->ip_info.ip));
        s_retry_num = 0;
        xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);
    }
}

void wifi_init_sta(void)
{
    s_wifi_event_group = xEventGroupCreate();

    ESP_ERROR_CHECK(esp_netif_init());

    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_sta();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    esp_event_handler_instance_t instance_any_id;
    esp_event_handler_instance_t instance_got_ip;
    ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT,
                                                        ESP_EVENT_ANY_ID,
                                                        &wifi_init_event_handler,
                                                        NULL,
                                                        &instance_any_id));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT,
                                                        IP_EVENT_STA_GOT_IP,
                                                        &wifi_init_event_handler,
                                                        NULL,
                                                        &instance_got_ip));

    wifi_config_t wifi_config = {
        .sta = {
            .ssid = STREAMS_POC_LIB_TEST_WIFI_SSID,
            .password = STREAMS_POC_LIB_TEST_WIFI_PASS,
            /* Authmode threshold resets to WPA2 as default if password matches WPA2 standards (pasword len => 8).
             * If you want to connect the device to deprecated WEP/WPA networks, Please set the threshold value
             * to WIFI_AUTH_WEP/WIFI_AUTH_WPA_PSK and set the password with length and format matching to
	     * WIFI_AUTH_WEP/WIFI_AUTH_WPA_PSK standards.
             */
            .threshold.authmode = ESP_WIFI_SCAN_AUTH_MODE_THRESHOLD,
            .sae_pwe_h2e = WPA3_SAE_PWE_BOTH,
        },
    };
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA) );
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config) );
    ESP_ERROR_CHECK(esp_wifi_start() );

    ESP_LOGI(TAG, "wifi_init_sta finished.");

    /* Waiting until either the connection is established (WIFI_CONNECTED_BIT) or connection failed for the maximum
     * number of re-tries (WIFI_FAIL_BIT). The bits are set by wifi_init_event_handler() (see above) */
    EventBits_t bits = xEventGroupWaitBits(s_wifi_event_group,
            WIFI_CONNECTED_BIT | WIFI_FAIL_BIT,
            pdFALSE,
            pdFALSE,
            portMAX_DELAY);

    /* xEventGroupWaitBits() returns the bits before the call returned, hence we can test which event actually
     * happened. */
    if (bits & WIFI_CONNECTED_BIT) {
        ESP_LOGI(TAG, "connected to ap SSID:%s password:%s",
                 STREAMS_POC_LIB_TEST_WIFI_SSID, STREAMS_POC_LIB_TEST_WIFI_PASS);
    } else if (bits & WIFI_FAIL_BIT) {
        ESP_LOGI(TAG, "Failed to connect to SSID:%s, password:%s",
                 STREAMS_POC_LIB_TEST_WIFI_SSID, STREAMS_POC_LIB_TEST_WIFI_PASS);
    } else {
        ESP_LOGE(TAG, "UNEXPECTED EVENT");
    }
}

void log_binary_data(const uint8_t *data, size_t length) {
    int i;
    for (i = 0; i < length; i++)
    {
        if (i > 0) printf(":");
        printf("%02X", data[i]);
    }
    printf("\n");
}


uint64_t get_base_mac_48_as_mocked_u64_dev_eui() {
    uint64_t mac_id;
    ESP_ERROR_CHECK(esp_efuse_mac_get_default((uint8_t*)&mac_id));
    ESP_LOGD(TAG, "[streams-poc-lib/main.c - fn get_mac_id_as_u64_dev_eui_mock()] mac_id as u64 is %" PRIu64 "\n", mac_id);
    return mac_id;
}

LoRaWanError send_request_via_socket(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback) {
    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_request_via_socket()] is called with %d bytes of request_data", length);

    log_binary_data(request_data, length);

    // Using LoraWAN the DevEUI will be available at the receiver side automatically. As we are using a wifi connection
    // instead we use the EUI-48 (formerly known as MAC-48) to mock the DevEUI.
    // Espressif provides a universally administered EUI-48 address (UAA) for each network interface controller (NIC).
    // E.g. WIFI, BT, ethernet, ...
    // to be independent from the used NIC we mock the LoraWAN DevEUI using the base MAC address that is used to generate
    // all other NIC specific MAC addresses.
    // https://docs.espressif.com/projects/esp-idf/en/v3.1.7/api-reference/system/base_mac_address.html
    //
    // To make sure the mocked LoraWAN DevEUI is received by the lora-app-srv-mock test application we will prepend the
    // request_data with the mocked_dev_eui.
    uint64_t mocked_dev_eui = get_base_mac_48_as_mocked_u64_dev_eui();
    assert(SEND_BUFFER_SIZE > (length + sizeof(uint64_t)));
    uint8_t send_buffer[SEND_BUFFER_SIZE];
    memcpy(send_buffer, &mocked_dev_eui, sizeof(uint64_t));
    size_t send_buffer_len = sizeof(uint64_t);
    memcpy(&send_buffer[send_buffer_len], request_data, length);
    send_buffer_len += length;

    int err = send(s_socket_handle, send_buffer, send_buffer_len, 0);
    if (err < 0) {
        ESP_LOGE(TAG, "Error occurred during sending: errno %d", errno);
        return LORAWAN_NO_CONNECTION;
    }

    uint8_t rx_buffer[2048];
    int rx_len = recv(s_socket_handle, rx_buffer, sizeof(rx_buffer), 0);
    // Error occurred during receiving
    if (rx_len < 0) {
        ESP_LOGE(TAG, "recv failed: errno %d", errno);
        return LORAWAN_NO_CONNECTION;
    }

    // Data received
    ESP_LOGI(TAG, "Received %d bytes from %s:", rx_len, STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS);
    log_binary_data(rx_buffer, rx_len);

    StreamsError streams_err = response_callback(rx_buffer, rx_len);
    if (streams_err < 0) {
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_request_via_socket()] response_callback returned with error code: %s, ", streams_error_to_string(streams_err));
    }

    // We arrived at this point so we assume that no LoRaWanError occurred.
    return LORAWAN_OK;
}

 int parse_lora_app_srv_mock_address(dest_addr_t *p_dest_addr) {
    struct http_parser_url parsed_url;
    http_parser_url_init(&parsed_url);

    const char* url_prefix = "http://";
    char app_srv_mock_address_as_url[256];
    strcpy(app_srv_mock_address_as_url, url_prefix);
    strcat(app_srv_mock_address_as_url, STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS);
    ESP_LOGD(TAG, "[streams-poc-lib/main.c - fn parse_lora_app_srv_mock_address()] app_srv_mock_address_as_url is '%s'", app_srv_mock_address_as_url);

    int parser_status = http_parser_parse_url(
        app_srv_mock_address_as_url,
        strlen(app_srv_mock_address_as_url),
        0,
        &parsed_url);

    if (parser_status != 0) {
        ESP_LOGE(TAG, "Error parse socket address %s", STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS);
        return ESP_ERR_INVALID_ARG;
    }

    char parsed_host[128];
    memset(parsed_host, '\0', sizeof(parsed_host));
    if (parsed_url.field_data[UF_HOST].len) {
        strncpy(
            parsed_host,
            app_srv_mock_address_as_url + parsed_url.field_data[UF_HOST].off,
            parsed_url.field_data[UF_HOST].len
        );
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn parse_lora_app_srv_mock_address()] parsed host is '%s'", parsed_host);
    } else {
        return ESP_ERR_INVALID_ARG;
    }

    char parsed_port[16];
    memset(parsed_port, '\0', sizeof(parsed_port));
    uint16_t parsed_port_u16;
    if (parsed_url.field_data[UF_PORT].len) {
        strncpy(
            parsed_port,
            app_srv_mock_address_as_url + parsed_url.field_data[UF_PORT].off,
            parsed_url.field_data[UF_PORT].len
        );
        parsed_port_u16 = parsed_url.port;
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn parse_lora_app_srv_mock_address()] parsed port string is '%s'. Port u16 = %d", parsed_port, parsed_port_u16);
    } else {
        return ESP_ERR_INVALID_ARG;
    }

#if defined(CONFIG_EXAMPLE_IPV4)
    p_dest_addr->sin_addr.s_addr = inet_addr(parsed_host);
    p_dest_addr->sin_family = AF_INET;
    p_dest_addr->sin_port = htons(parsed_port_u16);
#elif defined(CONFIG_EXAMPLE_IPV6)
    inet6_aton(host_ip, p_dest_addr->sin6_addr);
    p_dest_addr->sin6_family = AF_INET6;
    p_dest_addr->sin6_port = htons(parsed_port_u16);
    p_dest_addr->sin6_scope_id = esp_netif_get_netif_impl_index(EXAMPLE_INTERFACE);
#endif

    return 0;
}

int get_handle_of_prepared_socket(dest_addr_t *p_dest_addr)
{
    int addr_family = 0;
    int ip_protocol = 0;

    #if defined(CONFIG_EXAMPLE_IPV4)
        addr_family = AF_INET;
        ip_protocol = IPPROTO_IP;
    #elif defined(CONFIG_EXAMPLE_IPV6)
        addr_family = AF_INET6;
        ip_protocol = IPPROTO_IPV6;
    #endif

    int sock =  socket(addr_family, SOCK_STREAM, ip_protocol);
    if (sock < 0) {
        ESP_LOGE(TAG, "Unable to create socket: errno %d", errno);
        return sock;
    }
    ESP_LOGI(TAG, "Socket created, connecting to %s", STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS);

    int err = connect(sock, (struct sockaddr *)p_dest_addr, sizeof(dest_addr_t));
    if (err != 0) {
        ESP_LOGE(TAG, "Socket unable to connect: errno %d", errno);
        return sock;
    }
    ESP_LOGI(TAG, "Successfully connected");
    return sock;
}

void shut_down_socket(int sock_handle) {
    if (sock_handle != -1) {
        ESP_LOGE(TAG, "Shutting down socket");
        shutdown(sock_handle, 0);
        close(sock_handle);
    }
}

void prepare_socket_and_send_message(dest_addr_t* dest_addr ) {
    s_socket_handle = get_handle_of_prepared_socket(dest_addr);

    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Calling send_message for message_data of length %d \n\n", MESSAGE_DATA_LENGTH);
    send_message(message_data, MESSAGE_DATA_LENGTH, send_request_via_socket);

    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Shutting down socket");
    shut_down_socket(s_socket_handle);
}

void send_message_via_streams_poc_lib(void) {
    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Preparing WIFI");
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
      ESP_ERROR_CHECK(nvs_flash_erase());
      ret = nvs_flash_init();
    }

    ESP_LOGI(TAG, "ESP_WIFI_MODE_STA");
    wifi_init_sta();

    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Preparing netif and creating default event loop\n");
    ESP_ERROR_CHECK(esp_netif_init());

    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Preparing socket for future send_request_via_socket() calls");
#if defined(CONFIG_EXAMPLE_IPV4)
    dest_addr_t dest_addr;
#elif defined(CONFIG_EXAMPLE_IPV6)
    dest_addr_t dest_addr = { 0 };
#endif    
    
    if( 0 == parse_lora_app_srv_mock_address(&dest_addr) ) {
        while (1) {
            prepare_socket_and_send_message(&dest_addr);
            ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Waiting 5 seconds to send message again");
            sleep(5);
        }
    } else {
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn send_message_via_streams_poc_lib()] Could not parse address of lorawan application-server-mock");
    }
}

void app_main(void)
{
    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn app_main()] Sensor App is starting!\n");

    /* Print chip information */
    esp_chip_info_t chip_info;
    esp_chip_info(&chip_info);
    ESP_LOGI(TAG, "This is %s chip with %d CPU cores, WiFi%s%s, ",
            CONFIG_IDF_TARGET,
            chip_info.cores,
            (chip_info.features & CHIP_FEATURE_BT) ? "/BT" : "",
            (chip_info.features & CHIP_FEATURE_BLE) ? "/BLE" : "");

    ESP_LOGI(TAG, "silicon revision %d, ", chip_info.revision);

//    ESP_LOGI(TAG, "%dMB %s flash\n", spi_flash_get_chip_size() / (1024 * 1024),
//            (chip_info.features & CHIP_FEATURE_EMB_FLASH) ? "embedded" : "external");

    ESP_LOGI(TAG, "Free heap: %ld\n", esp_get_free_heap_size());

    if (is_streams_channel_initialized()) {
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn app_main()] Streams channel already initialized. Calling C function send_message_via_streams_poc_lib()");
        send_message_via_streams_poc_lib();
    } else {
        ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn app_main()] Streams channel for this sensor has not been initialized. Calling start_sensor_manager()");
        start_sensor_manager();
    }

    ESP_LOGI(TAG, "[streams-poc-lib/main.c - fn app_main()] Exiting Sensor App");
}