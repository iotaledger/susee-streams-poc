include(CMakePrintHelpers)

# If this component depends on other components - be it ESP-IDF or project-specific ones - enumerate those in the double-quotes below, separated by spaces
# Note that pthread should always be there, or else STD will not work
# Here's a non-minimal, reasonable set of ESP-IDF components that one might want enabled for Rust:
#    set(RUST_DEPS "pthread" "esp_http_client" "esp_http_server" "espcoredump" "app_update" "esp_serial_slave_link" "nvs_flash" "spi_flash" "esp_adc_cal" "mqtt")

set(RUST_DEPS "pthread" "driver" "espcoredump" "fatfs" "nvs_flash" "vfs" "esp_wifi" "esp_netif" "lwip" "esp_http_client" "wear_levelling" "esp_event" "esp_websocket_client")

idf_component_register(
    SRCS "placeholder.c"
    INCLUDE_DIRS ""
    PRIV_REQUIRES "${RUST_DEPS}"
)

if(CONFIG_IDF_TARGET_ARCH_RISCV)
    set(RUST_TARGET "riscv32imc-esp-espidf")
elseif(CONFIG_IDF_TARGET_ESP32)
    set(RUST_TARGET "xtensa-esp32-espidf")
elseif(CONFIG_IDF_TARGET_ESP32S2)
    set(RUST_TARGET "xtensa-esp32s2-espidf")
elseif(CONFIG_IDF_TARGET_ESP32S3)
    set(RUST_TARGET "xtensa-esp32s3-espidf")
else()
    message(FATAL_ERROR "Unsupported target ${CONFIG_IDF_TARGET}")
endif()

if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_BUILD_TYPE "debug")
    set(CARGO_BUILD_ARG "")
else()
    set(CARGO_BUILD_TYPE "release")
    set(CARGO_BUILD_ARG "--release")
endif()

# {% if std %}
set(CARGO_BUILD_STD_ARG -Zbuild-std=std,panic_abort -Zbuild-std-features=panic_immediate_abort)
# {% else %}
# set(CARGO_BUILD_STD_ARG -Zbuild-std=core,alloc,panic_abort -Zbuild-std-features=panic_immediate_abort)
# {% endif %}

set(CARGO_PROJECT_DIR "${CMAKE_CURRENT_LIST_DIR}")
set(CARGO_BUILD_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set(CARGO_TARGET_DIR "${CARGO_BUILD_DIR}/target")

set(RUST_INCLUDE_DIR "${CARGO_TARGET_DIR}")
set(RUST_STATIC_LIBRARY "${CARGO_TARGET_DIR}/${RUST_TARGET}/${CARGO_BUILD_TYPE}/libstreams_poc_lib.a")

# if this component uses CBindGen to generate a C header, uncomment the lines below and adjust the header name accordingly
set(RUST_INCLUDE_HEADER "${RUST_INCLUDE_DIR}/streams_poc_lib.h")
set_source_files_properties("${RUST_INCLUDE_HEADER}" PROPERTIES GENERATED true)

idf_build_get_property(sdkconfig SDKCONFIG)
idf_build_get_property(idf_path IDF_PATH)

cmake_print_variables(CARGO_PROJECT_DIR)
cmake_print_variables(CARGO_BUILD_DIR)
cmake_print_variables(CARGO_TARGET_DIR)
cmake_print_variables(RUST_INCLUDE_DIR)
cmake_print_variables(RUST_INCLUDE_HEADER)
cmake_print_variables(RUST_STATIC_LIBRARY)

cmake_print_variables(RUST_TARGET)
cmake_print_variables(sdkconfig)
cmake_print_properties(TARGETS ${COMPONENT_LIB} PROPERTIES INCLUDE_DIRECTORIES LINK_LIBRARIES)

ExternalProject_Add(
    streams_poc_lib_project
    PREFIX "${CARGO_PROJECT_DIR}"
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ${CMAKE_COMMAND} -E env
        CARGO_BUILD_TARGET=${RUST_TARGET}
        CARGO_BUILD_TARGET_DIR=${CARGO_TARGET_DIR}
        cargo clean --target ${RUST_TARGET} --target-dir ${CARGO_TARGET_DIR}
    USES_TERMINAL_BUILD true
    BUILD_COMMAND ${CMAKE_COMMAND} -E env
        CARGO_BUILD_TARGET=${RUST_TARGET}
        CARGO_BUILD_DIR=${RUST_BUILD_DIR}
        CARGO_BUILD_TARGET_DIR=${CARGO_TARGET_DIR}
        CARGO_CMAKE_BUILD_INCLUDES=$<TARGET_PROPERTY:${COMPONENT_LIB},INCLUDE_DIRECTORIES>
        CARGO_CMAKE_BUILD_LINK_LIBRARIES=$<TARGET_PROPERTY:${COMPONENT_LIB},LINK_LIBRARIES>
        CARGO_CMAKE_BUILD_SDKCONFIG=${sdkconfig}
        CARGO_CMAKE_BUILD_ESP_IDF=${idf_path}
        CARGO_CMAKE_BUILD_COMPILER=${CMAKE_C_COMPILER}
        cargo build --target ${RUST_TARGET} --target-dir ${CARGO_TARGET_DIR} ${CARGO_BUILD_ARG} ${CARGO_BUILD_STD_ARG} ${CARGO_FEATURES_ARG}
    INSTALL_COMMAND ""
    BUILD_ALWAYS TRUE
    TMP_DIR "${CARGO_BUILD_DIR}/tmp"
    STAMP_DIR "${CARGO_BUILD_DIR}/stamp"
    DOWNLOAD_DIR "${CARGO_BUILD_DIR}"
    SOURCE_DIR "${CARGO_PROJECT_DIR}"
    BINARY_DIR "${CARGO_PROJECT_DIR}"
    INSTALL_DIR "${CARGO_BUILD_DIR}"
    BUILD_BYPRODUCTS
        "${RUST_INCLUDE_HEADER}"
        "${RUST_STATIC_LIBRARY}"
)

add_prebuilt_library(streams_poc_lib_lib "${RUST_STATIC_LIBRARY}" PRIV_REQUIRES "${RUST_DEPS}" fatfs wpa_supplicant)
add_dependencies(streams_poc_lib_lib streams_poc_lib_project)

target_include_directories(${COMPONENT_LIB} PUBLIC "${RUST_INCLUDE_DIR}")
target_link_libraries(${COMPONENT_LIB} PRIVATE streams_poc_lib_lib)
