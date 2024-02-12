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

#include "esp_flash.h"
#include "esp_flash_spi_init.h"
#include "esp_partition.h"
#include "esp_vfs.h"
#include "esp_vfs_fat.h"

#include <http_parser.h>

#include "esp_chip_info.h"

#include "streams_poc_lib.h"

#include <inttypes.h>

// SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS
//      Callback driven, where the callback directly connects to the iota-bridge
//      via a WiFi connection controlled by the test app.
// SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK
//      Callback driven, where the callback uses the 'Application Server Connector Mock',
//      which is connected via a WiFi socket controlled by the test app.
// SMCT_LWIP:
//      Direct http communication between streams-poc-lib and iota-bridge
//      via a lwip connection provided by the test app. Currently a WiFi connect is used, but
//      other connections that support LWIP can be used equivalent.
// SMCT_STREAMS_POC_LIB_MANAGED_WIFI
//      Direct http communication between streams-poc-lib and iota-bridge via a WiFi
//      connection controlled by the streams-poc-lib.
typedef enum {
    SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS,
    SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK,
    SMCT_LWIP,
    SMCT_STREAMS_POC_LIB_MANAGED_WIFI,
} sensor_manager_connection_type_t;

/* ########################################################################################
   ############################ Test CONFIG ###############################################
   ######################################################################################## */

static const sensor_manager_connection_type_t SENSOR_MANAGER_CONNECTION_TYPE = SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS;

// Please edit your Wifi credentials here. Needed for Sensor initialization.
#define STREAMS_POC_LIB_TEST_WIFI_SSID "Susee Demo"
#define STREAMS_POC_LIB_TEST_WIFI_PASS "susee-rocks"
// The url of the iota-bridge to connect to. Needed for Sensor initialization.
#define STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL ("http://192.168.187.223:50000")
// IP address and port of the LoRaWAN AppServer Connector Mockup Tool to connect to.
// Needed for sending messages.
#define STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS ("192.168.187.223:50001")

#define SEND_MESSAGES_EVERY_X_SEC 5

// Setting STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH to NULL will make the streams-poc-lib
// using its own vfs_fat partition as been described in streams-poc-lib.h
// (sensor/streams-poc-lib/components/streams-poc-lib/include/streams-poc-lib.h)
//
// Specifying a STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH here will make the streams-poc-lib
// using a prepared file system. This test application can only handle vfs_fat base_path
// names so no subfolders are allowed.
// Example:
//          #define STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH ("/awesome-data")
#define STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH NULL

#define ESP_WIFI_SCAN_AUTH_MODE_THRESHOLD WIFI_AUTH_WPA2_PSK
#define STREAMS_POC_LIB_TEST_MAXIMUM_RETRY 5
#define SEND_BUFFER_SIZE 4096

/*Comment one of the following flags according to your STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL.
  BTW: The value of the flag is not of importance.*/
#define CONFIG_EXAMPLE_IPV4 true
// #define CONFIG_EXAMPLE_IPV6 true

/* ########################################################################################
   ############################ END of test CONFIG ########################################
   ######################################################################################## */

static const char *TAG = "test_streams_poc_lib";

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
#define WIFI_FAIL_BIT BIT1

static int s_retry_num = 0;

static int s_socket_handle;

static uint64_t s_mac_id = 0;

static bool s_perform_p_caller_user_data_test = false;


// This is the binary representation of the content of the file /test/meter_reading_1.json
#define MESSAGE_DATA_LENGTH 50 // 80 // 213
const uint8_t MESSAGE_DATA[MESSAGE_DATA_LENGTH] = {
    0x7b, 0x0a, 0x20, 0x20, 0x22, 0x74, 0x79, 0x70,
    0x65, 0x22, 0x3a, 0x20, 0x22, 0x6d, 0x65, 0x74,
    0x65, 0x72, 0x5f, 0x72, 0x65, 0x61, 0x64, 0x69,
    0x6e, 0x67, 0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22,
    0x72, 0x65, 0x67, 0x69, 0x73, 0x74, 0x65, 0x72,
    0x5f, 0x76, 0x61, 0x6c, 0x75, 0x65, 0x22, 0x3a,
    0x20, 0x32, // 0x32, 0x30, 0x31, 0x2e, 0x30, 0x32,      // 50 Bytes
    // 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x71, 0x75, 0x61,
    // 0x6c, 0x69, 0x66, 0x69, 0x65, 0x72, 0x22, 0x3a,
    // 0x20, 0x22, 0x61, 0x2d, 0x70, 0x6c, 0x75, 0x73,
    //        0x22, 0x2c, 0x0a, 0x20, 0x20, 0x22, 0x6f, 0x62,      // 80 Bytes
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

// Function that will be called in cb_fun_send_request_via_app_srv_connector_mock() to test p_caller_user_data
void test_p_caller_user_data(const char *function_name) {
    ESP_LOGI(TAG, "[fn test_p_caller_user_data] This function has successfully been called from within the %s() function", function_name);
}

// To test p_caller_user_data we can not use a raw function pointer because function pointers are handled differently compared
// to void pointers.
//
// https://stackoverflow.com/questions/13696918/c-cast-void-pointer-to-function-pointer
//      The C99 standard does not allow to convert between pointers to data (in the standard, “objects or
//      incomplete types” e.g. char* or void*) and pointers to functions.
//      ...  pointers to objects and pointers to functions do not have to be the same size.
//      On an example architecture, the former can be 64-bit and the latter 32-bit.
//
// Therefore we use a wrapper struct called test_p_caller_user_data_wrapper

typedef void (*test_p_caller_user_data_t)(const char *);

typedef struct test_p_caller_user_data_wrapper{
    test_p_caller_user_data_t fun_ptr;
} test_p_caller_user_data_wrapper_t;

static void wifi_init_event_handler(void *arg, esp_event_base_t event_base,
                                    int32_t event_id, void *event_data) {
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
    }
    else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        if (s_retry_num < STREAMS_POC_LIB_TEST_MAXIMUM_RETRY) {
            esp_wifi_connect();
            s_retry_num++;
            ESP_LOGI(TAG, "[fn wifi_init_event_handler] Retry to connect to the AP");
        }
        else {
            xEventGroupSetBits(s_wifi_event_group, WIFI_FAIL_BIT);
        }
        ESP_LOGI(TAG, "[fn wifi_init_event_handler] Connect to the AP fail");
    }
    else if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
        ESP_LOGI(TAG, "[fn wifi_init_event_handler] Got ip:" IPSTR, IP2STR(&event->ip_info.ip));
        s_retry_num = 0;
        xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);
    }
}

static bool mount_fatfs(wl_handle_t *p_wl_handle)
{
    ESP_LOGI(TAG, "[fn mount_fatfs] Mounting FAT filesystem with base_path '%s'", STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH);
    const esp_vfs_fat_mount_config_t mount_config = {
        .max_files = 4,
        .format_if_mount_failed = true,
        .allocation_unit_size = CONFIG_WL_SECTOR_SIZE};
    esp_err_t err = esp_vfs_fat_spiflash_mount(
        STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
        "storage",
        &mount_config,
        p_wl_handle);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "[fn mount_fatfs] Failed to mount FATFS (%s)", esp_err_to_name(err));
        return false;
    }
    return true;
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
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "[fn wifi_init_sta] wifi_init_sta finished.");

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
        ESP_LOGI(TAG, "[fn wifi_init_sta] connected to wifi SSID:%s password:%s",
                 STREAMS_POC_LIB_TEST_WIFI_SSID, STREAMS_POC_LIB_TEST_WIFI_PASS);
    }
    else if (bits & WIFI_FAIL_BIT) {
        ESP_LOGI(TAG, "[fn wifi_init_sta] Failed to connect to SSID:%s, password:%s",
                 STREAMS_POC_LIB_TEST_WIFI_SSID, STREAMS_POC_LIB_TEST_WIFI_PASS);
    }
    else {
        ESP_LOGE(TAG, "[fn wifi_init_sta] UNEXPECTED EVENT");
    }
}

void log_binary_data(const uint8_t *data, size_t length) {
    int i;
    for (i = 0; i < length; i++)
    {
        if (i > 0)
            printf(":");
        printf("%02X", data[i]);
    }
    printf("\n");
}

uint64_t get_base_mac_48_as_mocked_u64_dev_eui() {
    if (s_mac_id == 0) {
        ESP_ERROR_CHECK(esp_efuse_mac_get_default((uint8_t *)&s_mac_id));
        ESP_LOGD(TAG, "[fn get_base_mac_48_as_mocked_u64_dev_eui] s_mac_id as u64 is set to %" PRIu64 "\n", s_mac_id);
    } else {
        ESP_LOGD(TAG, "[fn get_base_mac_48_as_mocked_u64_dev_eui] returning initial s_mac_id %" PRIu64 "\n", s_mac_id);
    }

    return s_mac_id;
}

void get_base_mac_48_as_mocked_u64_dev_eui_string(char* p_buffer_len_128 ) {
        uint64_t mocked_dev_eui = get_base_mac_48_as_mocked_u64_dev_eui();
        sprintf(p_buffer_len_128, "%" PRIu64 "\0", mocked_dev_eui);
        ESP_LOGD(TAG, "[fn get_base_mac_48_as_mocked_u64_dev_eui_string] returning dev_eui_string %s", p_buffer_len_128);
}

LoRaWanError cb_fun_send_request_via_app_srv_connector_mock(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback, void *p_caller_user_data) {
    ESP_LOGI(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] is called with %d bytes of request_data", length);

    log_binary_data(request_data, length);

    // Using LoraWAN the DevEUI will be available at the receiver side automatically. As we are using a wifi connection
    // instead we use the EUI-48 (formerly known as MAC-48) to mock the DevEUI.
    // Espressif provides a universally administered EUI-48 address (UAA) for each network interface controller (NIC).
    // E.g. WIFI, BT, ethernet, ...
    // to be independent from the used NIC we mock the LoraWAN DevEUI using the base MAC address that is used to generate
    // all other NIC specific MAC addresses.
    // https://docs.espressif.com/projects/esp-idf/en/v3.1.7/api-reference/system/base_mac_address.html
    //
    // To make sure the mocked LoraWAN DevEUI is received by the app-srv-connector-mock test application we will prepend the
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
        ESP_LOGE(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] Error occurred during sending: errno %d", errno);
        return LORAWAN_NO_CONNECTION;
    }

    uint8_t rx_buffer[2048];
    int rx_len = recv(s_socket_handle, rx_buffer, sizeof(rx_buffer), 0);
    // Error occurred during receiving
    if (rx_len < 0) {
        ESP_LOGE(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] recv failed: errno %d", errno);
        return LORAWAN_NO_CONNECTION;
    }

    // Data received
    ESP_LOGI(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] Received %d bytes from %s:", rx_len, STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);
    log_binary_data(rx_buffer, rx_len);

    StreamsError streams_err = response_callback(rx_buffer, rx_len);
    if (streams_err < 0) {
        ESP_LOGI(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] response_callback returned with error code: %s, ", streams_error_to_string(streams_err));
    }

    // Before we leave this function we need to test the p_caller_user_data that should point to a test_p_caller_user_data_wrapper_t instance now
    if (s_perform_p_caller_user_data_test && (p_caller_user_data != NULL)) {
        test_p_caller_user_data_wrapper_t *callable = (test_p_caller_user_data_wrapper_t *)(p_caller_user_data);
        callable->fun_ptr("cb_fun_send_request_via_app_srv_connector_mock");
    }
    else if (s_perform_p_caller_user_data_test && (p_caller_user_data == NULL)) {
        ESP_LOGE(TAG, "[fn cb_fun_send_request_via_app_srv_connector_mock] p_caller_user_data has been set when the streams-poc-lib function was called, but now it's NULL");
    }

    // We arrived at this point so we assume that no LoRaWanError occurred.
    return LORAWAN_OK;
}

int parse_app_srv_connector_mock_address(dest_addr_t *p_dest_addr) {
    struct http_parser_url parsed_url;
    http_parser_url_init(&parsed_url);

    const char *url_prefix = "http://";
    char app_srv_connector_mock_address_as_url[256];
    strcpy(app_srv_connector_mock_address_as_url, url_prefix);
    strcat(app_srv_connector_mock_address_as_url, STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);
    ESP_LOGD(TAG, "[fn parse_app_srv_connector_mock_address] app_srv_connector_mock_address_as_url is '%s'", app_srv_connector_mock_address_as_url);

    int parser_status = http_parser_parse_url(
        app_srv_connector_mock_address_as_url,
        strlen(app_srv_connector_mock_address_as_url),
        0,
        &parsed_url);

    if (parser_status != 0) {
        ESP_LOGE(TAG, "[fn parse_app_srv_connector_mock_address] Error parse socket address %s", STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);
        return ESP_ERR_INVALID_ARG;
    }

    char parsed_host[128];
    memset(parsed_host, '\0', sizeof(parsed_host));
    if (parsed_url.field_data[UF_HOST].len) {
        strncpy(
            parsed_host,
            app_srv_connector_mock_address_as_url + parsed_url.field_data[UF_HOST].off,
            parsed_url.field_data[UF_HOST].len);
        ESP_LOGI(TAG, "[fn parse_app_srv_connector_mock_address] parsed host is '%s'", parsed_host);
    }
    else {
        return ESP_ERR_INVALID_ARG;
    }

    char parsed_port[16];
    memset(parsed_port, '\0', sizeof(parsed_port));
    uint16_t parsed_port_u16;
    if (parsed_url.field_data[UF_PORT].len) {
        strncpy(
            parsed_port,
            app_srv_connector_mock_address_as_url + parsed_url.field_data[UF_PORT].off,
            parsed_url.field_data[UF_PORT].len);
        parsed_port_u16 = parsed_url.port;
        ESP_LOGI(TAG, "[fn parse_app_srv_connector_mock_address] parsed port string is '%s'. Port u16 = %d", parsed_port, parsed_port_u16);
    }
    else {
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

void shut_down_socket(int sock_handle) {
    if (sock_handle != -1) {
        ESP_LOGI(TAG, "[fn shut_down_socket] Shutting down socket");
        shutdown(sock_handle, 0);
        close(sock_handle);
    }
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

    int sock = socket(addr_family, SOCK_STREAM, ip_protocol);
    if (sock < 0) {
        ESP_LOGE(TAG, "[fn get_handle_of_prepared_socket] Unable to create socket: errno %d", errno);
        return sock;
    }
    ESP_LOGI(TAG, "[fn get_handle_of_prepared_socket] Socket created, connecting to %s", STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);

    int err = connect(sock, (struct sockaddr *)p_dest_addr, sizeof(dest_addr_t));
    if (err != 0) {
        ESP_LOGE(TAG, "[fn get_handle_of_prepared_socket] Socket unable to connect the socket: errno %d", errno);
        shut_down_socket(sock);
        return -1;
    }
    ESP_LOGI(TAG, "[fn get_handle_of_prepared_socket] Successfully connected");
    return sock;
}

void prepare_socket_and_send_message_via_app_srv_connector_mock(dest_addr_t *p_dest_addr) {
    s_socket_handle = get_handle_of_prepared_socket(p_dest_addr);
    if (s_socket_handle > -1) {
        // Prepare testing p_caller_user_data
        test_p_caller_user_data_wrapper_t some_caller_user_data;
        some_caller_user_data.fun_ptr = &test_p_caller_user_data;
        s_perform_p_caller_user_data_test = true;

        ESP_LOGI(TAG, "[fn prepare_socket_and_send_message_via_app_srv_connector_mock] Calling send_message for MESSAGE_DATA of length %d \n\n", MESSAGE_DATA_LENGTH);
        send_message(MESSAGE_DATA, MESSAGE_DATA_LENGTH, cb_fun_send_request_via_app_srv_connector_mock, STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH, &some_caller_user_data);

        ESP_LOGI(TAG, "[fn prepare_socket_and_send_message_via_app_srv_connector_mock] Shutting down socket");
        shut_down_socket(s_socket_handle);
    }
}

void send_message_via_app_srv_connector_mock(dest_addr_t *p_dest_addr) {
    ESP_LOGI(TAG, "[fn send_message_via_app_srv_connector_mock] Sending messages using Application-Server-Connector-Mock: %s", STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);
    while (1) {
        prepare_socket_and_send_message_via_app_srv_connector_mock(p_dest_addr);
        ESP_LOGI(TAG, "[fn send_message_via_app_srv_connector_mock] Waiting 5 seconds to send message again");
        sleep(SEND_MESSAGES_EVERY_X_SEC);
    }
}

void init_sensor_via_app_srv_connector_mock(dest_addr_t *p_dest_addr) {
    ESP_LOGI(TAG, "[fn init_sensor_via_app_srv_connector_mock] Starting sensor_manager using Application-Server-Connector-Mock: %s", STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS);
    s_socket_handle = get_handle_of_prepared_socket(p_dest_addr);
    if (s_socket_handle > -1) {
        char dev_eui_buffer[128];
        get_base_mac_48_as_mocked_u64_dev_eui_string(dev_eui_buffer);
        start_sensor_manager(
            cb_fun_send_request_via_app_srv_connector_mock,
            dev_eui_buffer,
            STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
            NULL
        );

        ESP_LOGI(TAG, "[fn init_sensor_via_app_srv_connector_mock] Shutting down socket");
        shut_down_socket(s_socket_handle);
    }
}

void prepare_sensor_processing_via_app_srv_connector_mock(bool do_sensor_initialization) {
    #if defined(CONFIG_EXAMPLE_IPV4)
        dest_addr_t dest_addr;
    #elif defined(CONFIG_EXAMPLE_IPV6)
        dest_addr_t dest_addr = {0};
    #endif

    if (0 == parse_app_srv_connector_mock_address(&dest_addr)) {
        if (do_sensor_initialization) {
            init_sensor_via_app_srv_connector_mock(&dest_addr);
        } else {
            send_message_via_app_srv_connector_mock(&dest_addr);
        }
    }
    else {
        ESP_LOGI(TAG, "[fn prepare_sensor_processing_via_app_srv_connector_mock] Could not parse address of lorawan application-server-connector-mock");
    }
}

// -----------------------------------------------------------------------------------------

typedef struct {
    resolve_request_response_t response_callback;
    LoRaWanError status;
} http_response_recv_t;

void http_response_recv_receive_response(http_response_recv_t* p_receiver, uint16_t status, const uint8_t *body_bytes, size_t body_length) {
    if (status >= 200 && status < 300) {
        p_receiver->response_callback(body_bytes, body_length);
        p_receiver->status = LORAWAN_OK;
    }
    else {
        ESP_LOGE(TAG, "[fn http_response_recv_receive_response] Received HTPP error. Status: %d", status);
        p_receiver->status = LORAWAN_IOTA_BRIDGE_CONNECTOR_ERROR;
    }
}

void receive_http_response(uint16_t status, const uint8_t *body_bytes, size_t body_length, void *p_caller_user_data) {
    if (p_caller_user_data != NULL) {
        http_response_recv_t *p_receiver = (http_response_recv_t*)(p_caller_user_data);
        http_response_recv_receive_response(p_receiver, status, body_bytes, body_length);
    }
    else {
        ESP_LOGE(TAG, "[fn receive_http_response] p_caller_user_data is NULL");
    }
}

LoRaWanError send_request_via_wifi(const uint8_t *request_data, size_t length, resolve_request_response_t response_callback, void *p_caller_user_data) {
    iota_bridge_tcpip_proxy_options_t iota_bridge_proxy_opt = {
        .dev_eui = get_base_mac_48_as_mocked_u64_dev_eui(),
        .iota_bridge_url = STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL
    };

    http_response_recv_t response_receiver = {
        .response_callback = response_callback
    };

    post_binary_request_to_iota_bridge(request_data, length, &iota_bridge_proxy_opt, receive_http_response, &response_receiver);
    return response_receiver.status;
}

// --------------------------------------------------------------------------------------------------------------

void init_sensor_via_callback_io(void) {
    switch (SENSOR_MANAGER_CONNECTION_TYPE) {
        case SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS: {
            ESP_LOGI(TAG, "[fn init_sensor_via_callback_io] Starting sensor_manager using IOTA-Bridge: %s", STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL);
            char dev_eui_buffer[128];
            get_base_mac_48_as_mocked_u64_dev_eui_string(dev_eui_buffer);
            start_sensor_manager(
                send_request_via_wifi,
                dev_eui_buffer,
                STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
                NULL
            );
        }
        break;
        case SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK:
            prepare_sensor_processing_via_app_srv_connector_mock(true);
        break;
        default:
            ESP_LOGE(TAG, "[fn init_sensor_via_callback_io] Unexpected SENSOR_MANAGER_CONNECTION_TYPE %d", SENSOR_MANAGER_CONNECTION_TYPE);
        break;
    }
}

void prepare_lwip_socket_based_sensor_processing(bool do_sensor_initialization) {
    ESP_LOGI(TAG, "[fn prepare_lwip_socket_based_sensor_processing] Preparing WIFI");
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }

    ESP_LOGI(TAG, "[fn prepare_lwip_socket_based_sensor_processing] ESP_WIFI_MODE_STA");
    wifi_init_sta();

    ESP_LOGI(TAG, "[fn prepare_lwip_socket_based_sensor_processing] Preparing netif and creating default event loop\n");
    ESP_ERROR_CHECK(esp_netif_init());

    if (do_sensor_initialization) {
        switch (SENSOR_MANAGER_CONNECTION_TYPE) {
            case SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS:   // No break
            case SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK:
                init_sensor_via_callback_io();
            break;
            case SMCT_LWIP: {
                ESP_LOGI(TAG, "[fn prepare_lwip_socket_based_sensor_processing] Calling start_sensor_manager_lwip() without WiFi credentials");
                char dev_eui_buffer[128];
                get_base_mac_48_as_mocked_u64_dev_eui_string(dev_eui_buffer);
                start_sensor_manager_lwip(
                    STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL,
                    dev_eui_buffer,
                    STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
                    NULL,
                    NULL
                );
            }
            break;
            default:
                ESP_LOGE(TAG, "[fn prepare_lwip_socket_based_sensor_processing] Unexpected SENSOR_MANAGER_CONNECTION_TYPE %d", SENSOR_MANAGER_CONNECTION_TYPE);
            break;
        }
    }
    else {
        ESP_LOGI(TAG, "[fn prepare_lwip_socket_based_sensor_processing] Preparing socket for future cb_fun_send_request_via_app_srv_connector_mock() calls");
        prepare_sensor_processing_via_app_srv_connector_mock(false);
    }
}

void process_test() {
    if (is_streams_channel_initialized(STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH)) {
        ESP_LOGI(TAG, "[fn process_test] Streams channel already initialized. Going to send messages every %d seconds", SEND_MESSAGES_EVERY_X_SEC);
        prepare_lwip_socket_based_sensor_processing(false);
    }
    else {
        ESP_LOGI(TAG, "[fn process_test] Streams channel for this sensor has not been initialized. Going to initialize the sensor");
        switch (SENSOR_MANAGER_CONNECTION_TYPE) {
            case SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS:   // No break
            case SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK:  // No break
            case SMCT_LWIP: {
                ESP_LOGI(TAG, "[fn process_test] Calling prepare_lwip_socket_based_sensor_processing() to use start_sensor_manager() later on");
                prepare_lwip_socket_based_sensor_processing(true);
            }
            break;
            case SMCT_STREAMS_POC_LIB_MANAGED_WIFI: {
                ESP_LOGI(TAG, "[fn process_test] Calling start_sensor_manager_lwip() using WiFi managed by the streams-poc-lib.");
                char dev_eui_buffer[128];
                get_base_mac_48_as_mocked_u64_dev_eui_string(dev_eui_buffer);
                start_sensor_manager_lwip(
                    STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL,
                    dev_eui_buffer,
                    STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
                    STREAMS_POC_LIB_TEST_WIFI_SSID,
                    STREAMS_POC_LIB_TEST_WIFI_PASS
                );
            }
            break;
            default:
                ESP_LOGE(TAG, "[fn process_test] Unknown SENSOR_MANAGER_CONNECTION_TYPE %d", SENSOR_MANAGER_CONNECTION_TYPE);
            break;
        }
    }
}

void app_main(void) {
    ESP_LOGI(TAG, "[fn app_main] Sensor App is starting!\n");

    /* Print chip information */
    esp_chip_info_t chip_info;
    esp_chip_info(&chip_info);
    ESP_LOGI(TAG, "[fn app_main] This is %s chip with %d CPU cores, WiFi%s%s, ",
             CONFIG_IDF_TARGET,
             chip_info.cores,
             (chip_info.features & CHIP_FEATURE_BT) ? "/BT" : "",
             (chip_info.features & CHIP_FEATURE_BLE) ? "/BLE" : ""
    );

    ESP_LOGI(TAG, "[fn app_main] silicon revision %d, ", chip_info.revision);

    //    ESP_LOGI(TAG, "[fn app_main] %dMB %s flash\n", spi_flash_get_chip_size() / (1024 * 1024),
    //            (chip_info.features & CHIP_FEATURE_EMB_FLASH) ? "embedded" : "external");

    ESP_LOGI(TAG, "[fn app_main] Free heap: %ld\n", esp_get_free_heap_size());

    wl_handle_t wl_handle = WL_INVALID_HANDLE;
    if (STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH) {
        mount_fatfs(&wl_handle);
    }

    process_test();

    if (STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH != NULL && wl_handle != WL_INVALID_HANDLE) {
        ESP_LOGI(TAG, "[fn app_main] unmounting vfs_fat");
        ESP_ERROR_CHECK(esp_vfs_fat_spiflash_unmount(STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH, wl_handle));
    }

    ESP_LOGI(TAG, "[fn app_main] Exiting Sensor App");
}