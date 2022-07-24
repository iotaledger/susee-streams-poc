include(CMakePrintHelpers)

idf_component_register(
    SRCS "placeholder.c"
    INCLUDE_DIRS ""
    PRIV_REQUIRES "${RUST_DEPS}"
)

set(CARGO_BUILD_TYPE "release")
set(CARGO_BUILD_ARG "--release")
if(CONFIG_IDF_TARGET_ARCH_RISCV)
    set(CARGO_TARGET "riscv32imc-esp-espidf")
    set(CARGO_FEATURES_ARG "")
elseif(CONFIG_IDF_TARGET_ARCH_XTENSA)
    set(CARGO_TARGET "xtensa-esp32-espidf")
    set(CARGO_FEATURES_ARG "--features=std")
endif()

set(RUST_PROJECT_DIR "${CMAKE_CURRENT_LIST_DIR}")
set(RUST_BUILD_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set(RUST_TARGET_DIR "${RUST_BUILD_DIR}/target")
set(RUST_INCLUDE_DIR "${RUST_TARGET_DIR}")
set(RUST_INCLUDE_HEADER "${RUST_INCLUDE_DIR}/streams_poc_lib.h")
set(RUST_STATIC_LIBRARY "${RUST_TARGET_DIR}/${CARGO_TARGET}/${CARGO_BUILD_TYPE}/libstreams_poc_lib.a")

idf_build_get_property(sdkconfig SDKCONFIG)

cmake_print_variables(RUST_PROJECT_DIR)
cmake_print_variables(RUST_BUILD_DIR)
cmake_print_variables(RUST_TARGET_DIR)
cmake_print_variables(RUST_INCLUDE_DIR)
cmake_print_variables(RUST_INCLUDE_HEADER)
cmake_print_variables(RUST_STATIC_LIBRARY)

cmake_print_variables(CARGO_TARGET)
cmake_print_variables(sdkconfig)
cmake_print_properties(TARGETS ${COMPONENT_LIB} PROPERTIES INCLUDE_DIRECTORIES LINK_LIBRARIES)

ExternalProject_Add(
    streams_poc_lib_project
    PREFIX "${RUST_PROJECT_DIR}"
    DOWNLOAD_COMMAND ""
    CONFIGURE_COMMAND ${CMAKE_COMMAND} -E env
        CARGO_BUILD_TARGET=${CARGO_TARGET}
        CARGO_BUILD_TARGET_DIR=${RUST_TARGET_DIR}
        cargo clean
    BUILD_COMMAND ${CMAKE_COMMAND} -E env
        CARGO_CMAKE_BUILD_ESP_IDF=${IDF_PATH}
        CARGO_BUILD_TARGET=${CARGO_TARGET}
        CARGO_BUILD_DIR=${RUST_BUILD_DIR}
        CARGO_BUILD_TARGET_DIR=${RUST_TARGET_DIR}
        CARGO_CMAKE_BUILD_INCLUDES=$<TARGET_PROPERTY:${COMPONENT_LIB},INCLUDE_DIRECTORIES>
        CARGO_CMAKE_BUILD_LINK_LIBRARIES=$<TARGET_PROPERTY:${COMPONENT_LIB},LINK_LIBRARIES>
        CARGO_CMAKE_BUILD_SDKCONFIG=${sdkconfig}
        CARGO_CMAKE_BUILD_COMPILER=${CMAKE_C_COMPILER}
        cargo build ${CARGO_BUILD_ARG} ${CARGO_FEATURES_ARG}
    INSTALL_COMMAND ""
    BUILD_ALWAYS TRUE
    TMP_DIR "${RUST_BUILD_DIR}/tmp"
    STAMP_DIR "${RUST_BUILD_DIR}/stamp"
    DOWNLOAD_DIR "${RUST_BUILD_DIR}"
    SOURCE_DIR "${RUST_PROJECT_DIR}"
    BINARY_DIR "${RUST_PROJECT_DIR}"
    INSTALL_DIR "${RUST_BUILD_DIR}"
    BUILD_BYPRODUCTS
        "${RUST_INCLUDE_HEADER}"
        "${RUST_STATIC_LIBRARY}"
)

set_source_files_properties("${RUST_INCLUDE_HEADER}" PROPERTIES GENERATED true)

add_prebuilt_library(streams_poc_lib_lib "${RUST_STATIC_LIBRARY}" PRIV_REQUIRES "${RUST_DEPS}" fatfs wpa_supplicant)
add_dependencies(streams_poc_lib_lib streams_poc_lib_project)

target_include_directories(${COMPONENT_LIB} PUBLIC "${RUST_INCLUDE_DIR}")
target_link_libraries(${COMPONENT_LIB} PRIVATE streams_poc_lib_lib)
