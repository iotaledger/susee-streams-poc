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

To build the test application and the *streams-poc-lib* for ESP32 platforms (currently only ESP32-C3 provided),
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

#### Configuring the test application 

The *Test CONFIG* section at the top of `main.c` file includes several
precompiler macro definitions and static constants that need to be configured.

**Sensor-Manager Connection**

The static constant `SENSOR_MANAGER_CONNECTION_TYPE` controls how
the test application connects the iota-bridge.

The [start_sensor_manager()](components/streams-poc-lib/include/streams_poc_lib.h)
and `start_sensor_manager_lwip()` functions provide several binary i/o and
[Espressif LWIP stack](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-guides/lwip.html)
options. The test application therefore provides different
connection types to test these options.

Depending on the connection type, the
*IOTA Bridge* is connected directly via a WiFi socket (managed
by the test application or by the *streams-poc-lib*) or by the
[*AppServer Connector Mockup Tool*](../../app-srv-connector-mock).
In case a connection type is based on *callback i/o*, the *streams-poc-lib*
interface uses callback functions to transfer the payload data to the 
*streams-poc-lib test application*. The test application then transfers
the payload data to the *IOTA Bridge* or *AppServer Connector Mockup Tool*
via a WiFi socket connection.

Choose one of the following connection types:

* SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS<br>
  **Only available for Sensor-Initialization**<br>
  Callback driven, where the callback directly connects to 
  the *IOTA Bridge* via a WiFi connection
  controlled by the *Sensor* test app.
* SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK<br>
  **Available for: Sensor-Initialization and Send-Message processing**<br>
  Callback driven, where the callback uses the *Application Server Connector Mock*
  which is connected via a WiFi socket controlled by the *Sensor* test app.
  This connection type uses the same callback functions that are used in the
  [initialized mode](#initialized-mode) to mock the LoRaWAN network. 
* SMCT_LWIP<br>
  **Only available for Sensor-Initialization**<br>
  Direct http communication between the *streams-poc-lib* and the
  *IOTA Bridge* via a lwip connection provided
  by the *Sensor* test app.
* SMCT_STREAMS_POC_LIB_MANAGED_WIFI<br>
  **Only available for Sensor-Initialization**<br>
  Direct http communication between the *streams-poc-lib* and the
  *IOTA Bridge* via a WiFi connection controlled by
  the *streams-poc-lib*.

Following connection types can only be used for Sensor-Initialization:
 * SMCT_CALLBACK_DIRECT_IOTA_BRIDGE_ACCESS
 * SMCT_LWIP
 * SMCT_STREAMS_POC_LIB_MANAGED_WIFI

This is because a Sensor is expected to send messages via an
Application-Server-Connector in real world scenarios.
Therefore, only SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK
can be used for Send-Message processing
([Initialized Mode](initialized-mode)).

Sensor-Initialization may happen in a location where WiFi is available.
Therefore, the above listed connection types can be used for
Sensor-Initialization
[Uninitialized Mode](#uninitialized-mode).

**WiFi**

The *streams-poc-lib test Application* always communicates via WiFi sockets to
mock the missing LoRaWAN network, so you'll need to specify WiFi credentials
for your test environment:
```` C
  #define STREAMS_POC_LIB_TEST_WIFI_SSID "Susee Demo"
  #define STREAMS_POC_LIB_TEST_WIFI_PASS "susee-rocks"
````
**Needed Services**

Additionally you need to specify at least one of the following ip addresses
(both are recommended) to allow the test application to connect to the
*IOTA Bridge* or [*AppServer Connector Mockup Tool*](../../app-srv-connector-mock).
Please replace the preconfigured ip addresses (`192.168.47.11` in the example below)
with the ip address of the device that runs the
specific service. Please do not edit the port numbers.

```` C
#define STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL ("http://192.168.47.11:50000")
#define STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS ("192.168.47.11:50001")
````

**Streams Client Data Storage and VFS-FAT Management**

The test application needs to store data on the *Sensor* device.
This can be done fully automatically by the streams-poc-lib,
or can be controlled and processed by
the *Sensor* application (in our case the streams-poc-lib test
application).

The static constant `STREAMS_CLIENT_DATA_STORAGE_TYPE` defines how streams client data 
shall be stored.

Choose one of these options:
* CLIENT_DATA_STORAGE_VFS_FAT<br>
  Streams client data are stored in the vfs_fat data partition
  managed by the *streams-poc-lib* or by the test application,
  according to the used `VFS_FAT_MANAGEMENT` option.
* CLIENT_DATA_STORAGE_CALL_BACK<br>
  Storage of the streams client data is fully managed by the
  test application:
  * initial streams client data are provided by the application
    via an initial data buffer.
  * after the streams client data have changed, the resulting
    latest data are handed to the application via a callback
    function (provided by the test application) that is called
    by the streams-poc-lib.
    
See the
[streams-poc-lib header file](components/streams-poc-lib/include/streams_poc_lib.h)
for more details about the interface.

The static constant `VFS_FAT_MANAGEMENT` controls how vfs_fat data 
partitions, needed to store files in spiflash, shall be managed.

Choose one of these options:

* VFS_FAT_STREAMS_POC_LIB_MANAGED<br>
  The *streams-poc-lib* will initialize and use its default
  '/spiflash' data partition.
  To use this option, the default 'storage' data partition
  needs to be configured in the 'partitions.scv' file of the
  applications build project.
* VFS_FAT_APPLICATION_MANAGED<br>
  The Sensor application using the streams-poc-lib functions
  is responsible to manage a vfs_fat data partition.

See the documentation in the
[streams-poc-lib header file](components/streams-poc-lib/include/streams_poc_lib.h)
for more details and needed preconditions.

## Using the test application

The test application runs in two different modes:
* Uninitialized
* Initialized

If the *streams-poc-lib test application* has not been *initialized*
before it will run in the *Uninitialized mode*
until it is successfully initialized and powered down.

After the *initialization* and a reboot of the device the test application
will run in the *Initialized mode*.

Additionally to the background information provided here,
you can find usage examples in the
[test README](../../test#sensor-initialization) file.

### Uninitialized mode
In the Uninitialized mode the application behaves
like *x86/PC Sensor* application when the 
[`--act-as-remote-controlled-sensor` CLI argument](../../sensor/README.md#remote-control-cli-commands)
is used. In this mode the *Management Console* can be used to initialize the sensor.

This is done using the `--init-sensor` option of the *Management Console*
([project directory](../../management-console))
which uses the *IOTA Bridge* application ([project directory](../../iota-bridge)) to remote control
the *Sensor*.

After you've built the *Management Console* and the *IOTA Bridge* as being
described in the [main README.md file of the susee-streams-poc repository](../../README.md#build)
please follow the instructions of the *Automatic Sensor Initialization* section of the
[test README](../../test/README.md#automatic-sensor-initialization---streams-poc-lib-test-application).

For more details regarding the *Automatic Sensor Initialization*
process have a look into the
[*Management Console* --init-sensor documentation](../../management-console/README.md#automatic-sensor-initialization)
.

### Initialized mode
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

In the followimg example we start both applications on the same machine in
separate shells (additionally to the following example you can also follow the
test description, provided in the
[test README](../../test#send-messages-using-the-sensor) file).

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
via 192.168.47.11:50001. For the *streams-poc-lib* test application,
this has been defined at compile time using the 
`STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS` #define in the [main.c](./main/main.c) file.

Please make sure for the *Uninitialized mode* to set
`SENSOR_MANAGER_CONNECTION_TYPE` to `SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`
as described 
[above](#streams-client-data-storage-and-vfs-fat-management).

### Sensor Initialization vs Reinitialization

As the [DevEUI](../README.md#mocked-deveuis) of the *streams-poc-lib test application* is derived from the 
[base MAC address of the ESP32 MCU](../README.md#mocked-deveuis)
the DevEUI will remain the same even if the device flash storage has
been erased.

The difference between an [initialization](../../README.md#initialization) 
and [reinitialization](../../README.md#sensor-reinitialization) using the
*streams-poc-lib test application* just includes that a new
[random seed](../../README.md#common-file-persistence)
is generated and the [initialization count](../README.md#initialization-count)
is set to zero.

As a consequence of the *initialization count* reset and the static DevEUI, an
*IOTA Bridge* having cached an outdated *IOTA Streams* channel id
with *initialization count* zero, has no possibility
to detect that the channel id has been changed. Therefore, the 
*streams-poc-lib test application* should only be initialized once.

If the *Streams* channel needs to be replaced after the *initialization*,
a *reinitialization* should be processed. Otherwise, in case the device flash has
been erased, the databases of all deployed
*IOTA Bridge* instances need to be cleared, or the specific dataset in these
*IOTA Bridge* databases needs to be updated/deleted using the *IOTA Bridge*
[lorawan-node API endpoint](../../iota-bridge/README.md#lorawan-node-endpoints).

To avoid the above described problem a *Sensor Reinitialization* must be done if a
DevEUI is maintained while the *IOTA Streams Channel*-id is changed.

Unfortuantely, using the current version of the *streams-poc-lib test application*
the *Sensor Reinitialization* cannot be tested using.

_Nevertheless, a reinitialization of your *Sensor* Application using the
*Streams POC Library*, can be achieved easily:_

* *Sensor* Applications using CLIENT_DATA_STORAGE_CALL_BACK for *Streams Client Data
  Storage*, can implement a *Sensor Reinitialization* by deleting the
  existing initial streams client data and by handing an empty
  ([streams_client_data_persistence_t.latest_client_data_bytes](./components/streams-poc-lib/include/streams_poc_lib.h)).
  buffer to the `prepare_client_data_storage___call_back___...` function
  before the `start_sensor_manager()` function is called.

* *Sensor* Applications using CLIENT_DATA_STORAGE_VFS_FAT for *Streams Client Data
  Storage*, can implement a *Sensor Reinitialization*, if the file
  used to store the Streams client state is deleted.