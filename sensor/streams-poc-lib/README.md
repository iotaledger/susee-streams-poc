# Susee - Streams POC Library

The *streams-poc-lib* provides C bindings for all functions needed in the SUSEE-Module (a.k.a. *Sensor*) to
use IOTA Streams for energy meter data transmissions.

This project contains a basic unit test like `src/main.c` file, and the *streams-poc-lib* RUST library project.

The API of the *streams-poc-lib* can be found in the file `components/streams-poc-lib/include/streams_poc_lib.h`

An already build static library file for ESP32-C3 can be found here: `components/streams-poc-lib/lib/libstreams_poc_lib.a`

The `main.c` file is build using the Espressif IDF (esp-idf) build process. The *streams-poc-lib* is build via cargo
(RUST build tool and package manager) which is integrated into the CMake files of the esp-idf build system.
Using the Espressif build utils (e.g. *idf.py*) you don't need to care about the integrated RUST cargo build process. 

The *streams-poc-lib* project is located in the `components/streams_poc_lib` directory.

## About
This project is based on the
[esp-idf-template](https://github.com/esp-rs/esp-idf-template/blob/master/README-cmake.md)
which is a successor of the original [rust-esp32-example by Espressif](https://github.com/espressif/rust-esp32-example).

Especially the CMake integration has been taken from the
[esp-idf-template CMakeLists.txt](https://github.com/esp-rs/esp-idf-template/blob/master/cmake/components/rust-%7B%7Bproject-name%7D%7D/CMakeLists.txt)

## Prerequisites for building for ESP32

Please follow the instructions given in the
["Prerequisites / For ESP32" section of the main README file](../../README.md#for-esp32) in the root 
folder of this repository. 

You can also follow the [esp-idf-template Readme](https://github.com/esp-rs/esp-idf-template/blob/master/README-cmake.md)
for mixed Rust/C ESP-IDF projects (driven by idf.py and CMake).

Please checkout the "*tag:v5.1-dev*"
[version of the esp-idf build tools](https://github.com/espressif/esp-idf/releases/tag/v5.1-dev)
using the 
[git-workflow](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/versions.html#git-workflow)
.

## Build for ESP32

After having installed the "*tag:v5.1-dev*"
[version of the esp-idf build tools](https://github.com/espressif/esp-idf/releases/tag/v5.1-dev)
you are ready to start building.

Please note that the "*tag:v5.1-dev*" is also used in the

* sensor/main-rust-esp-rs/.cargo/config.toml
* sensor/streams-poc-lib/.cargo/config.toml

for the ESP_IDF_VERSION environment variable. 

To build the `main.c` file and *streams-poc-lib* you will need to do the following:
```bash
get_idf
export SENSOR_MAIN_POC_WIFI_SSID=NameOfMyWifiGoesHere
export SENSOR_MAIN_POC_WIFI_PASS=SecureWifiPassword
export SENSOR_MAIN_POC_IOTA_BRIDGE_URL="http://192.168.47.11:50000" 
idf.py flash monitor
``` 

## Initializing a sensor

This is done using the *Management Console* application ([project directory](../../management-console)) and the
*IOTA Bridge* application ([project directory](../../iota-bridge)). After you build these applications as being
described in the [main README.md file of the susee-streams-poc repository](https://github.com/iotaledger/susee-streams-poc)
please follow the instructions of the
<a href="https://github.com/iotaledger/susee-streams-poc#sensor-initialization">Sensor Initialization</a> section
to fully automatically initialize a *ESP32 Sensor*.