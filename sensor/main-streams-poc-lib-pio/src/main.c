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

#include "streams_poc_lib.h"

#include <inttypes.h>

/* ########################################################################################
   ############################ Test CONFIG ###############################################
   ######################################################################################## */

// Please edit your Wifi credentials here
#define STREAMS_POC_LIB_TEST_WIFI_SSID "Susee Demo"
#define STREAMS_POC_LIB_TEST_WIFI_PASS "susee-rocks"
#define STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS ("192.168.0.100:50001")

#include "../../streams-poc-lib/main/main.c"