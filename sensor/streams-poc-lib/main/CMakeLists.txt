idf_component_register(SRCS "main.c"
                    INCLUDE_DIRS "")

if(DEFINED ENV{SENSOR_MAIN_POC_WIFI_SSID})
    add_compile_definitions(STREAMS_POC_LIB_TEST_WIFI_SSID="$ENV{SENSOR_MAIN_POC_WIFI_SSID}")
else()
    message(FATAL_ERROR "You need to define ebvironment variable SENSOR_MAIN_POC_WIFI_SSID")
endif()

if(DEFINED ENV{SENSOR_MAIN_POC_WIFI_PASS})
    add_compile_definitions(STREAMS_POC_LIB_TEST_WIFI_PASS="$ENV{SENSOR_MAIN_POC_WIFI_PASS}")
else()
    message(FATAL_ERROR "You need to define ebvironment variable SENSOR_MAIN_POC_WIFI_PASS")
endif()

if(DEFINED ENV{SENSOR_STREAMS_POC_LIB_LORA_APP_SRV_MOCK_ADDRESS})
    add_compile_definitions(STREAMS_POC_LIB_TEST_LORA_APP_SRV_MOCK_ADDRESS="$ENV{SENSOR_STREAMS_POC_LIB_LORA_APP_SRV_MOCK_ADDRESS}")
else()
    message(FATAL_ERROR "You need to define ebvironment variable SENSOR_STREAMS_POC_LIB_LORA_APP_SRV_MOCK_ADDRESS")
endif()