# Susee - Streams POC Library

The *streams-poc-lib* provides C bindings for all functions needed in the SUSEE-Module (a.k.a. *Sensor*) to
use IOTA Streams for energy meter data transmissions.

This project contains the *streams-poc-lib* RUST library and a test application - implemented in the `src/main.c` file - to test the library functionality
using a WIFI socket instead of a LoRaWAN connection

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
export SENSOR_STREAMS_POC_LIB_LORA_APP_SRV_MOCK_ADDRESS="192.168.47.11:50001" 
idf.py flash monitor
``` 
Please replace the ip address in the above given example with the ip address of the machine
that runs the specific application (*Iota Bridge* & *LoraWan AppServer Mockup Tool*).

## Using the test application

The test application runs in two different modes:
* Uninitialized
* Initialized

In the **Uninitialized** mode the application behaves like the standalone *ESP32 Sensor* application. In this mode
the *Management Console* or *Sensor Remote Cotrol* application can be used to initialize the sensor. 

The easiest way is to use the `--init-sensor` option of the *Management Console* ([project directory](../../management-console))
which uses the *IOTA Bridge* application ([project directory](../../iota-bridge)) to remote control the Sensor.

After you've built the *Management Console* and *IOTA Bridge* as being
described in the [main README.md file of the susee-streams-poc repository](../../README.md#build)
please follow the instructions of the *Automatic Sensor Initialization* section of the
[README](../../README.md#automatic-sensor-initialization)
to initialize the Sensor.

In the **Initialized** mode the application sends an example message using the 'send_message()' function of the
*streams-poc-lib*. This is followed by calls of the 'send_request_via_lorawan_t' and 'resolve_request_response_t'
functions as they have been declared in the 'streams_poc_lib.h' interface and as they are defined in the `src/main.c` file.

To run the test application in the *Initialized* mode you need to run the *LoraWan AppServer Mockup Tool*
which will mock the behavior of the LoRaWan Application Server.
Have a look into the [*LoraWan AppServer Mockup Tool README*](../../lora-app-srv-mock/README.md) for further details.

Additionally the *Iota Bridge* is needed. Start both applications in the `target/release` folder.
We recommend to use only binaries build in release mode because the needed *proof of work* needs a lot of time
if applications are build in debug mode.

In this example we start both applications on the same machine in separate shells.

In the first shell:
```bash
    > ./lora-app-srv-mock -l 192.168.47.11:50001
    > Listening on: 192.168.47.11:50001
```

In the second shell:

```bash
    > /iota-bridge
    > [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > Listening on http://127.0.0.1:50000
```

*Iota Bridge* and *LoraWan AppServer Mockup Tool* are communicating via 127.0.0.1:50000 which is the default value
for both applications. 

The *streams-poc-lib* test application will communicate with the *LoraWan AppServer Mockup Tool*
via 192.168.47.11:50001. Please note that, for the *streams-poc-lib* test application this has been defined at compile
time using the `SENSOR_STREAMS_POC_LIB_LORA_APP_SRV_MOCK_ADDRESS` environment variable as been described above.
 