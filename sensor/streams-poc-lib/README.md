# Susee - Streams POC Library

The *streams-poc-lib* provides C bindings for all functions needed in the SUSEE-Module (a.k.a. *Sensor*) to
use IOTA Streams for energy meter data transmissions.

This project contains the *streams-poc-lib* RUST library and a test application - implemented in the
[main.c](./main/main.c) file - to test the library functionality
using a WIFI socket instead of a LoRaWAN connection

The API of the *streams-poc-lib* can be found in the file `components/streams-poc-lib/include/streams_poc_lib.h`

An already build static library file for ESP32-C3 can be found here: `components/streams-poc-lib/lib/libstreams_poc_lib.a`

The [main.c](./main/main.c) file is build using the Espressif IDF (esp-idf) build process. The *streams-poc-lib* is build via cargo
(RUST build tool and package manager) which is integrated into the CMake files of the esp-idf build system.
Using the Espressif build utils (e.g. *idf.py*) you don't need to care about the integrated RUST cargo build process. 

The *streams-poc-lib* project is located in the `components/streams_poc_lib` directory.

## About
This project is based on the
[esp-idf-template](https://github.com/esp-rs/esp-idf-template/blob/master/README-cmake.md)
which is a successor of the original [rust-esp32-example by Espressif](https://github.com/espressif/rust-esp32-example).

Especially the CMake integration has been taken from the
[esp-idf-template CMakeLists.txt](https://github.com/esp-rs/esp-idf-template/blob/master/cmake/components/rust-%7B%7Bproject-name%7D%7D/CMakeLists.txt)

## Prerequisites

To build the test application and the *Streams POC library* for ESP32 platforms (currently only ESP32-C3 provided),
you need to install the Espressif software development environment called esp-idf. 

These are the main steps to install the Espressif software development environment:
  * Please follow the *Espressif Install Guide* for
    [manual instalation](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/linux-macos-setup.html)
    or via [IDE install](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/index.html#ide)
    for the ESP32-C3 - master branch(latest).
  * If you have not flashed an ESP32 application before you should also follow the
    [First Steps on ESP-IDF](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/linux-macos-setup.html#get-started-first-steps)
    section of the 
    [Espressif Get Startet](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/index.html#) guide
  * You should also [Check your serial port on Linux and macOS](https://docs.espressif.com/projects/esp-idf/en/latest/esp32c3/get-started/establish-serial-connection.html#check-port-on-linux-and-macos)
    to find out how to access the serial port connection to the ESP32.
    Please replace the port identifier `/dev/ttyYOURPORT`
    used in the README files of this repository always with your port identifier.

You can also follow the [esp-idf-template Readme](https://github.com/esp-rs/esp-idf-template/blob/master/README-cmake.md)
for mixed Rust/C ESP-IDF projects (driven by idf.py and CMake).

Please checkout "release/v4.4.3"
[version of the esp-idf build tools](https://github.com/espressif/esp-idf/releases/tag/v4.4.3)
using the 
[git-workflow](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/versions.html#git-workflow)
.

## Build

After having installed the "*release/v4.4.3*"
[version of the esp-idf build tools](https://github.com/espressif/esp-idf/releases/tag/v4.4.3)
you are ready to start building.

Please note that the "*release/v4.4.3*" is also used in the

* sensor/main-rust-esp-rs/.cargo/config.toml
* sensor/streams-poc-lib/.cargo/config.toml

for the ESP_IDF_VERSION environment variable. 

To build the [main.c](./main/main.c) file and *streams-poc-lib* you will need to do the following:
```bash
get_idf
idf.py flash monitor
``` 
The ip address, eventually needed WiFi credentials and several other test options
can be configured in the `Test CONFIG` section at the top of the [main.c](./main/main.c) file.

## Using the test application

The test application runs in two different modes:
* Uninitialized
* Initialized

##### Uninitialized mode
In the Uninitialized mode the application behaves like the standalone *ESP32 Sensor* application. 
It communicates directly with the *IOTA Bridge* using a WiFi socket connection that is managed
by the *streams-poc-lib* [start_sensor_manager()](components/streams-poc-lib/include/streams_poc_lib.h)
function.

In this mode the *Management Console* or *Sensor Remote Cotrol* application can be used to initialize the sensor.

The easiest way is to use the `--init-sensor` option of the *Management Console* ([project directory](../../management-console))
which uses the *IOTA Bridge* application ([project directory](../../iota-bridge)) to remote control the Sensor.

After you've built the *Management Console* and *IOTA Bridge* as being
described in the [main README.md file of the susee-streams-poc repository](../../README.md#build)
please follow the instructions of the *Automatic Sensor Initialization* section of the
[README](../../README.md#automatic-sensor-initialization)
to initialize the Sensor.

The test application provides different connection types for the *Sensor* to *IOTA Bridge* connection
in the uninitialized mode:
* CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS<br>
  Callback driven, where the callback directly connects to the *IOTA Bridge* via a WiFi connection
  controlled by the *Sensor* test app.
* CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK<br>
  Callback driven, where the callback uses the *Application Server Connector Mock* which is connected
  via a WiFi socket controlled by the *Sensor* test app.
* LWIP<br>
  Direct http communication between the *streams-poc-lib* and the *IOTA Bridge* via a lwip connection provided
  by the *Sensor* test app.
* STREAMS_POC_LIB_MANAGED_WIFI<br>
  Direct http communication between the *streams-poc-lib* and the *IOTA Bridge* via a WiFi connection
  controlled by the *streams-poc-lib*.

The connection type for the uninitialized mode can be set in the `Test CONFIG` section of the
[main.c](./main/main.c) file.

##### Initialized mode
In the Initialized mode the application sends an example message using the 'send_message()' function of the
*streams-poc-lib*. This is followed by calls of the 'send_request_via_lorawan_t' and 'resolve_request_response_t'
functions as they have been declared in the 'streams_poc_lib.h' interface and as they are defined in the
[main.c](./main/main.c) file.

To run the test application in the *Initialized* mode you need to run the *AppServer Connector Mockup Tool*
which will mock the behavior of the LoRaWan Application Server.
Have a look into the [*AppServer Connector Mockup Tool README*](../../app-srv-connector-mock/README.md) for further details.

Additionally the *Iota Bridge* is needed. Start both applications in the `target/release` folder.
We recommend to use only binaries build in release mode because the needed *proof of work* needs a lot of time
if applications are build in debug mode.

In this example we start both applications on the same machine in separate shells.

In the first shell:
```bash
    > ./app-srv-connector-mock -l 192.168.47.11:50001
    > Listening on: 192.168.47.11:50001
```

In the second shell:

```bash
    > /iota-bridge
    > [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > Listening on http://127.0.0.1:50000
```

*Iota Bridge* and *AppServer Connector Mockup Tool* are communicating via 127.0.0.1:50000 which is the default value
for both applications. 

The *streams-poc-lib* test application will communicate with the *AppServer Connector Mockup Tool*
via 192.168.47.11:50001. Please note that, for the *streams-poc-lib* test application this has been
defined at compile time using a 
`STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS` #define in the [main.c](./main/main.c) file.

The connection type settings for the uninitialized mode described above are ignored in the initialized mode.