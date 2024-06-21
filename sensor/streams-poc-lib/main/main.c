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

// Defines how the Sensor application connects to the iota-bridge.
// Following connection types can only be used for Sensor-Initialization:
//  * SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS
//  * SMCT_LWIP
//  * SMCT_STREAMS_POC_LIB_MANAGED_WIFI
//
// This is because a Sensor is expected to send messages via an
// Application-Server-Connector in real world scenarios.
// Therefore only SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK
// can be used for Send-Message processing.
//
// Sensor-Initialization may happen in a location where WiFi is available.
// Therefore the above listed connection types can be used for
// Sensor-Initialization.
//
// SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS
//      ------- Only available for Sensor-Initialization ------
//      Callback driven, where the callback directly connects to the iota-bridge
//      via a WiFi connection controlled by the test app.
// SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK
//      Available for: Sensor-Initialization and Send-Message processing.
//      Callback driven, where the callback uses the 'Application Server Connector Mock',
//      which is connected via a WiFi socket controlled by the test app.
// SMCT_LWIP:
//      ------- Only available for Sensor-Initialization ------
//      Direct http communication between streams-poc-lib and iota-bridge
//      via a lwip connection provided by the test app. Currently a WiFi connection is used, but
//      other connections that support LWIP can be used equivalent.
// SMCT_STREAMS_POC_LIB_MANAGED_WIFI
//      ------- Only available for Sensor-Initialization ------
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

// Choose which connection type shall be used to connect the iota-bridge
static const sensor_manager_connection_type_t SENSOR_MANAGER_CONNECTION_TYPE = SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK;

// Please edit your Wifi credentials here. Needed for Sensor initialization.
#define STREAMS_POC_LIB_TEST_WIFI_SSID "Susee Demo"
#define STREAMS_POC_LIB_TEST_WIFI_PASS "susee-rocks"
// The url of the iota-bridge to connect to. Needed for Sensor initialization.
#define STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL ("http://192.168.0.100:50000")
// IP address and port of the LoRaWAN AppServer Connector Mockup Tool to connect to.
// Needed for sending messages.
#define STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS ("192.168.0.100:50001")

#define SEND_MESSAGES_EVERY_X_SEC 5

// Defines how streams client data shall be stored
// Possible values: [CLIENT_DATA_STORAGE_VFS_FAT, CLIENT_DATA_STORAGE_CALL_BACK]
// More details can be found in sensor/streams-poc-lib/components/streams-poc-lib/include/streams-poc-lib.h
static const StreamsClientDataStorageType s_streams_client_data_storage_type = CLIENT_DATA_STORAGE_CALL_BACK;

// Defines how vfs_fat data partitions, needed to store files in spiflash, shall be managed
// Possible values: [VFS_FAT_STREAMS_POC_LIB_MANAGED, VFS_FAT_APPLICATION_MANAGED]
// More details can be found in sensor/streams-poc-lib/components/streams-poc-lib/include/streams-poc-lib.h
static const VfsFatManagement s_vfs_fat_management = VFS_FAT_APPLICATION_MANAGED;

// In case s_vfs_fat_management == VFS_FAT_APPLICATION_MANAGED the following macro defines
// the path used as param vfs_fat_path for the prepare_client_data_storage___vfs_fat___application_managed()
// function call.
// This test application will prepare a file system which is used by the streams_poc_lib.
// This test application can only handle vfs_fat base_path names, so no subfolders are allowed.
#define STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH ("/awesome-data")

// In case s_streams_client_data_storage_type == CLIENT_DATA_STORAGE_CALL_BACK,
// this test application will store the Streams client state data in a file
// with the following filename.
#define STREAMS_CLIENT_DATA_FILE_NAME ("/client-data.bin")


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

// Several static variables needed for the streams-client-data callback
static char s_streams_client_data_file_name_buf[256];
static uint8_t s_streams_client_data_buffer[2048];
static size_t s_streams_client_data_buffer_len;
static bool s_application_managed_fat_fs_is_mounted = false;

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

// --------------------------------------------------------------------------------------------------------------

void set_streams_client_data_buffer(
   const uint8_t *client_data_bytes,
   size_t client_data_bytes_length
) {
    memcpy(s_streams_client_data_buffer, client_data_bytes, client_data_bytes_length);
    s_streams_client_data_buffer_len = client_data_bytes_length;
    ESP_LOGI(TAG, "[fn set_streams_client_data_buffer()] Copied client_data_bytes into local data buffer. length: %i", client_data_bytes_length);
}

const char *get_vfs_fat_base_path() {
    switch (s_vfs_fat_management) {
     case VFS_FAT_STREAMS_POC_LIB_MANAGED:
        return get_vfs_fat_mount_base_path();
     case VFS_FAT_APPLICATION_MANAGED:
        return STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH;
     default:
        ESP_LOGE(TAG, "[fn prepare_streams_client_data_file_name()] s_vfs_fat_management value is not allowed: %d", s_vfs_fat_management);
        return NULL;
    }
}

bool streams_client_data_update_call_back(
    const uint8_t *client_data_bytes,
    size_t client_data_bytes_length,
    void *p_caller_user_data
) {
    if (client_data_bytes_length > 0) {
        ESP_LOGI(TAG, "[fn streams_client_data_update_call_back()] Opening file for writing: %s", s_streams_client_data_file_name_buf);
        FILE *f = fopen(s_streams_client_data_file_name_buf, "wb+");
        if (f == NULL) {
            ESP_LOGE(TAG, "[fn streams_client_data_update_call_back()] Failed to open file for writing %s", s_streams_client_data_file_name_buf);
            return false;
        }
        // Step 1: Write size of client_data_bytes to file
        size_t bytes_written = fwrite(&client_data_bytes_length, sizeof(size_t), 1, f);
        if (bytes_written <= 0) {
            return false;
        };
        // Step 2: Write client_data_bytes to file
        bytes_written = fwrite(client_data_bytes, 1, client_data_bytes_length, f);
        if (bytes_written <= 0) {
            return false;
        };
        int success_is_zero = fclose(f);
        if (success_is_zero != 0) {
            return false;
        }
        ESP_LOGI(TAG, "[fn streams_client_data_update_call_back()] Wrote client_data_bytes into file. client_data_bytes_length: %i", client_data_bytes_length);
    } else {
        ESP_LOGI(TAG, "[fn streams_client_data_update_call_back()] client_data_bytes_length == 0. Removing file %s", s_streams_client_data_file_name_buf);
        int success_is_zero = remove(s_streams_client_data_file_name_buf);
        if (success_is_zero < 0) {
            ESP_LOGD(TAG, "[fn streams_client_data_update_call_back()] remove file failed, probably the file didn't exist");
        }
    }

    ESP_LOGI(TAG, "[fn streams_client_data_update_call_back()] Updating s_streams_client_data_buffer" );
    set_streams_client_data_buffer(client_data_bytes, client_data_bytes_length);
    return true;
}

void unmout_vfs_fat(wl_handle_t wl_handle) {
    if (s_application_managed_fat_fs_is_mounted && wl_handle != WL_INVALID_HANDLE) {
        ESP_LOGI(TAG, "[fn unmout_vfs_fat] unmounting vfs_fat");
        ESP_ERROR_CHECK(esp_vfs_fat_spiflash_unmount(get_vfs_fat_base_path(), wl_handle));
        s_application_managed_fat_fs_is_mounted = false;
    }
}

static bool mount_fatfs(wl_handle_t *p_wl_handle)
{
    if (false == s_application_managed_fat_fs_is_mounted) {
        const char* base_path = get_vfs_fat_base_path();
        ESP_LOGI(TAG, "[fn mount_fatfs] Mounting FAT filesystem with base_path '%s'", base_path);
        const esp_vfs_fat_mount_config_t mount_config = {
            .max_files = 4,
            .format_if_mount_failed = true,
            .allocation_unit_size = CONFIG_WL_SECTOR_SIZE};
        esp_err_t err = esp_vfs_fat_spiflash_mount(
            base_path,
            "storage",
            &mount_config,
            p_wl_handle);
        if (err != ESP_OK) {
            ESP_LOGE(TAG, "[fn mount_fatfs] Failed to mount FATFS (%s)", esp_err_to_name(err));
            return false;
        }
        s_application_managed_fat_fs_is_mounted = true;
        return true;
    } else {
        ESP_LOGW(TAG, "[fn mount_fatfs] Function mount_fatfs() should not be called if s_application_managed_fat_fs_is_mounted == false");
        return false;
    }
}

int read_streams_client_data_from_file() {
    wl_handle_t wl_handle = WL_INVALID_HANDLE;
    if (false == s_application_managed_fat_fs_is_mounted) {
        ESP_LOGI(TAG, "[fn read_streams_client_data_from_file()] Mounting FatFs");
        mount_fatfs(&wl_handle);
    } else {
        ESP_LOGI(TAG, "[fn read_streams_client_data_from_file()] FatFs is already mounted");
    }

    s_streams_client_data_buffer_len = 0;
    ESP_LOGI(TAG, "[fn read_streams_client_data_from_file()] Opening file for reading: %s", s_streams_client_data_file_name_buf);
    FILE *f = fopen(s_streams_client_data_file_name_buf, "rb");
    if (f == NULL) {
        ESP_LOGE(TAG, "[fn read_streams_client_data_from_file()] Failed to open file for reading %s", s_streams_client_data_file_name_buf);
        unmout_vfs_fat(wl_handle);
        return 0;
    }
    // Step 1: Read size of client_data_bytes
    size_t client_data_bytes_length;
    int bytes_read = fread(&client_data_bytes_length, sizeof(size_t), 1, f);
    if (bytes_read == 0) {
        ESP_LOGE(TAG, "[fn read_streams_client_data_from_file()] File is empty. Returning 0.");
        unmout_vfs_fat(wl_handle);
        return 0;
    };
    if (bytes_read < 0) {
        ESP_LOGE(TAG, "[fn read_streams_client_data_from_file()] fread() failed. Returning error.");
        unmout_vfs_fat(wl_handle);
        return bytes_read;
    };
    ESP_LOGI(TAG, "[fn read_streams_client_data_from_file()] Reading %d bytes of streams-client-data", client_data_bytes_length);
    // Step 2: Read client_data_bytes
    bytes_read = fread(s_streams_client_data_buffer, 1, client_data_bytes_length, f);
    if (bytes_read != client_data_bytes_length) {
        ESP_LOGE(TAG, "[fn read_streams_client_data_from_file()] bytes_read != client_data_bytes_length. Returning error.\n  bytes_read: %d \n  client_data_bytes_length: %d",
        bytes_read, client_data_bytes_length);
        unmout_vfs_fat(wl_handle);
        return -1;
    };
    ESP_LOGD(TAG, "[fn read_streams_client_data_from_file()] Successfully read %d bytes", bytes_read);
    int success_is_zero = fclose(f);
    unmout_vfs_fat(wl_handle);
    if (success_is_zero != 0) {
        ESP_LOGE(TAG, "[fn read_streams_client_data_from_file()] fclose() failed. Returning error.");
        return -1;
    }
    s_streams_client_data_buffer_len = bytes_read;
    ESP_LOGI(TAG, "[fn read_streams_client_data_from_file()] Read client_data_bytes into local data buffer. client_data_bytes_length: %i",
        s_streams_client_data_buffer_len
    );
    return s_streams_client_data_buffer_len;
}

// --------------------------------------------------------------------------------------------------------------

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
        sprintf(p_buffer_len_128, "%" PRIX64 "\0", mocked_dev_eui);
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

size_t prepare_streams_client_data_file_name() {
    const char *base_path = get_vfs_fat_base_path();
    strcpy(s_streams_client_data_file_name_buf, base_path);
    strcat(s_streams_client_data_file_name_buf, STREAMS_CLIENT_DATA_FILE_NAME);
    return strlen(s_streams_client_data_file_name_buf);
}

bool prepare_client_data_persistence_for_call_back_usage(streams_client_data_persistence_t *p_streams_client_data_persistence) {
    int bytes_read_or_success = read_streams_client_data_from_file();
    if (bytes_read_or_success < 0) {
        ESP_LOGE(TAG, "[fn prepare_client_data_persistence_for_call_back_usage] read_streams_client_data_from_file() returned error");
        return false;
    }
    bool is_sensor_initialized = (bytes_read_or_success > 0);
    ESP_LOGD(TAG, "[fn prepare_client_data_persistence_for_call_back_usage] is_sensor_initialized: %i", is_sensor_initialized);

    switch (s_vfs_fat_management) {
        case VFS_FAT_STREAMS_POC_LIB_MANAGED:
            ESP_LOGD(TAG, "[fn prepare_client_data_persistence_for_call_back_usage] case VFS_FAT_STREAMS_POC_LIB_MANAGED");
            return prepare_client_data_storage___call_back___streams_poc_lib_managed_vfs_fat(
                p_streams_client_data_persistence,
                is_sensor_initialized,
                s_streams_client_data_buffer,
                s_streams_client_data_buffer_len,
                streams_client_data_update_call_back,
                NULL
            );
        case VFS_FAT_APPLICATION_MANAGED:
            ESP_LOGD(TAG, "[fn prepare_client_data_persistence_for_call_back_usage] case VFS_FAT_APPLICATION_MANAGED");
            bool result = prepare_client_data_storage___call_back___application_managed_vfs_fat(
                p_streams_client_data_persistence,
                STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH,
                is_sensor_initialized,
                s_streams_client_data_buffer,
                s_streams_client_data_buffer_len,
                streams_client_data_update_call_back,
                NULL
            );
            ESP_LOGD(TAG, "[fn prepare_client_data_persistence_for_call_back_usage] result is %d", result);
            return result;
        default:
            ESP_LOGE(TAG, "[fn prepare_client_data_persistence] Unknown s_streams_client_data_storage_type: %d",
                s_streams_client_data_storage_type);
   }
   return false;
}

bool prepare_client_data_persistence(streams_client_data_persistence_t *p_streams_client_data_persistence) {
    if (prepare_streams_client_data_file_name() <= 0) {
        return false;
    }

    switch (s_streams_client_data_storage_type) {
        case CLIENT_DATA_STORAGE_VFS_FAT: {
            switch (s_vfs_fat_management) {
                case VFS_FAT_STREAMS_POC_LIB_MANAGED:
                    return prepare_client_data_storage___vfs_fat___streams_poc_lib_managed(p_streams_client_data_persistence);
                case VFS_FAT_APPLICATION_MANAGED:
                    return prepare_client_data_storage___vfs_fat___application_managed(
                        p_streams_client_data_persistence,
                        STREAMS_POC_LIB_TEST_VFS_FAT_BASE_PATH
                    );
                default:
                    ESP_LOGE(TAG, "[fn prepare_client_data_persistence] Unknown s_streams_client_data_storage_type: %d",
                        s_streams_client_data_storage_type);
            }
        }
        break;
        case CLIENT_DATA_STORAGE_CALL_BACK:{
            return prepare_client_data_persistence_for_call_back_usage(p_streams_client_data_persistence);
       }
       default:
            ESP_LOGE(TAG, "[fn prepare_client_data_persistence] Unknown s_streams_client_data_storage_type: %d",
                s_streams_client_data_storage_type);
    }
    return false;
}

void prepare_socket_and_send_message_via_app_srv_connector_mock(dest_addr_t *p_dest_addr) {
    s_socket_handle = get_handle_of_prepared_socket(p_dest_addr);
    if (s_socket_handle > -1) {
        // Prepare testing p_caller_user_data
        test_p_caller_user_data_wrapper_t some_caller_user_data;
        some_caller_user_data.fun_ptr = &test_p_caller_user_data;
        s_perform_p_caller_user_data_test = true;
        streams_client_data_persistence_t client_data_persistence;
        bool succes = prepare_client_data_persistence(&client_data_persistence);
        if (succes) {
            ESP_LOGI(TAG, "[fn prepare_socket_and_send_message_via_app_srv_connector_mock] Calling send_message for MESSAGE_DATA of length %d \n\n", MESSAGE_DATA_LENGTH);
            send_message(
                MESSAGE_DATA,
                MESSAGE_DATA_LENGTH,
                cb_fun_send_request_via_app_srv_connector_mock,
                &client_data_persistence,
                &some_caller_user_data
            );
        } else {
            ESP_LOGI(TAG, "[fn prepare_socket_and_send_message_via_app_srv_connector_mock] prepare_client_data_persistence had no Success");
        }

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
        streams_client_data_persistence_t client_data_persistence;
        bool succes = prepare_client_data_persistence(&client_data_persistence);
        if (succes) {
            start_sensor_manager(
                cb_fun_send_request_via_app_srv_connector_mock,
                dev_eui_buffer,
                &client_data_persistence,
                NULL
            );
        }
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
    char dev_eui_buffer[128];
    get_base_mac_48_as_mocked_u64_dev_eui_string(dev_eui_buffer);

    iota_bridge_tcpip_proxy_options_t iota_bridge_proxy_opt = {
        .dev_eui = dev_eui_buffer,
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
            streams_client_data_persistence_t client_data_persistence;
            bool succes = prepare_client_data_persistence(&client_data_persistence);
            if (succes) {
                start_sensor_manager(
                    send_request_via_wifi,
                    dev_eui_buffer,
                    &client_data_persistence,
                    NULL
                );
            };
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
                streams_client_data_persistence_t client_data_persistence;
                bool success = prepare_client_data_persistence(&client_data_persistence);
                if (success) {
                    start_sensor_manager_lwip(
                        STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL,
                        dev_eui_buffer,
                        &client_data_persistence,
                        NULL,
                        NULL
                    );
                }
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
    ESP_LOGD(TAG, "[fn process_test] Starting");
    streams_client_data_persistence_t client_data_persistence;
    bool success = prepare_client_data_persistence(&client_data_persistence);
    if (!success) {
        return;
    }
    if (is_streams_channel_initialized(&client_data_persistence)) {
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
                    &client_data_persistence,
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
    if (s_vfs_fat_management == VFS_FAT_APPLICATION_MANAGED) {
        mount_fatfs(&wl_handle);
    }

    process_test();

    if (s_vfs_fat_management == VFS_FAT_APPLICATION_MANAGED && wl_handle != WL_INVALID_HANDLE) {
        unmout_vfs_fat(wl_handle);
    }

    ESP_LOGI(TAG, "[fn app_main] Exiting Sensor App");
}