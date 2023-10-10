# Tests

This folder contains documentation and resources to facilitate manual and automated tests. 
The documentation includes step by step descriptions of all tasks needed to perform manual
tests.  

Here is an overview of the contained sub-folders:

* [scripts](./scripts)<br>
  Contains script files to perform automatic
  tests for the *SUSEE Streams POC* applications and libraries. Have a look into the
  [test scripts README](./scripts/README.md) for more details.
* [iota-bridge](./iota-bridge) <br>
  Contains several curl scripts for manual testing of the *IOTA Bridge* API endpoints
  that are dedicated for public use.
  Have a look into the [*IOTA Bridge* README](../iota-bridge/README.md) for more details.
* [payloads](./payloads)<br>
  Contains several message files that can be used to test the *Sensors* send
  functionality (--file-to-send` argument of the *Sensor* application CLI).
  See [below](#send-messages-using-the-sensor) for more details.
  
The *Sensor* test applications can be tested manually by using the CLI of the applications.
This is described in the [Sensor Initialization](#sensor-initialization)
and [Send messages using the Sensor](#send-messages-using-the-sensor)
section below.

Please note that the tests provided here underlie
[several restrictions](../README.md#restrictions-of-the-provided-tests) that are described in
the main README.

Please also note that you may also process the tests described in this README,
using the docker images for the SUSEE applications as been described in the
[docker folder](../docker/README.md).

## Test workspace
As all built applications are located in the `target/debug` or `target/release`
sub-folder of the *susee-streams-poc* repository root folder, the easiest way
to run the tests described below is, to use one
of these folders as *test workspace* for manual testing.

We recommend using the release build of the applications because the proof of work,
done in the *IOTA Bridge*, is very time-consuming otherwise.

In the following test descriptions we presume that the working directory of the
command shell is the folder where all used test applications are stored. This is
called *test workspace* in the following. For example, to use the `target/release`
folder as *test workspace* you need to open a command shell and enter
```bash
    > cd ~/path-to-my-susee-streams-poc-repository/target/release
```  

Feel free to use a different folder as *test workspace*.
Just copy the needed test applications into the folder before you start your tests.
All files created by the test applications will be created locally in the
working directory from where the application has been started.
Following the definition of our *test workspace*, this is the folder where the applications
are stored.

## Sensor Initialization
There are two ways to initialize a *Sensor*. The easiest way is to use the
[`--init-sensor` option](../management-console/README.md#automatic-sensor-initialization)
of the *Management Console* application which will perform an automatic *Sensor* initialization.

If you prefer to have more insights into the initialization process you can do the *Sensor* initialization
manually, using the *Management Console* application CLI.

Depending on the *Sensor* app (x86/PC, ESP32 Sensor, streams-poc-lib test application) the steps
to initialize the sensor are different. In the following sections the *Sensor* initialization is therefore
described for each *Sensor* application seperately.

Here are some general hints and aspects that apply to all *Sensor* applications:

* As described [above](#test-workspace) we recommend using the
  `target/release` folder as *test workspace*.
* Initialization vs. Reinitialization<br>
  In the tests described below we will do a *Sensor* initialization and therefore
  we will make sure that the filesystem used by the *Sensor* app does not contain
  *IOTA Streams* user state and wallet files. Have a look into the
  [Sensor README](../sensor/README.md#initialization-count), the
  [initialization](../README.md#initialization) and
  [reinitialization](../README.md#sensor-reinitialization) workflow description
  for more details regarding the differences between those two workflows.<br>
  In the [Sensor Reinitialization](#sensor-reinitialization) section the *reinitialization*
  of the *streams-poc-lib test application* and the *x86/PC Sensor* is described.<br>
  In the [streams-poc-lib README](../sensor/streams-poc-lib/README.md#sensor-initialization-vs-reinitialization)
  you will also find more details about the special handling of *Sensor* applications
  using real LoRaWAN DevEUIs.
* In case a SUSEE POC application is listening to an external ip address,
  the example ip `192.168.47.11` is used in the test descriptions below.
  Please replace the ip address with the ip address of the network interface of your computer.
  You need also to make sure, the used port is opened in the firewall of your OS.
  After having started the application (e.g. the *IOTA-Bridge*) you can use telnet from another
  machine in your LAN to verify that the application can be accessed
  from within the LAN.

### Automatic Sensor Initialization

To automatically initialize a *Sensor* we need to use the
[`--init-sensor` option](../management-console/README.md#automatic-sensor-initialization)
of the *Management Console* application.
As the *Sensor* applications communicate with the *Management Console* via the *IOTA Bridge*
we need to start three applications.

*Management Console* and *IOTA Bridge* are started in their own command shells and will run
in parallel. If you use the *x86/PC Sensor*, it will be launched in an additional
command shell running parallel to the other two programs. In case a Sensor application
running on an ESP32 device is used, the log monitor utility will run in the third command shell.

#### Automatic Sensor Initialization - streams-poc-lib test application

To perform the initialization you'll need the *Espressif IDF SDK* installed. Please have a look into
the [streams-poc-lib README prerequisites section](../sensor/streams-poc-lib#prerequisites) for further details.

In the following test steps we asume that the `SENSOR_MANAGER_CONNECTION_TYPE` in the test application
[main.c file](../sensor/streams-poc-lib/main/main.c) has **not** been set to
`SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`. Using one of the other *sensor manager connection types*,
we don't need to run the [*AppServer Connector Mockup Tool*](../app-srv-connector-mock)
because the *streams-poc-lib* directly communicates with the *IOTA Bridge* as been described
in the [*streams-poc-lib* README](../sensor/streams-poc-lib#using-the-test-application).

If `SENSOR_MANAGER_CONNECTION_TYPE` has been set to `SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`
the *AppServer Connector Mockup Tool* needs to be run in an additional command shell which is described
in a [separated section](#automatic-sensor-initialization---streams-poc-lib-test-application-with-appserver-connector)
below.

As the test application always uses WiFi to connect to the LAN you will also
need to define the following precompiler macros in the test application
[main.c](../sensor/streams-poc-lib/main/main.c) file:
* STREAMS_POC_LIB_TEST_WIFI_SSID
* STREAMS_POC_LIB_TEST_WIFI_PASS,
* STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL
* STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS

Please have a look at the `Test CONFIG` section of the 
[main.c](../sensor/streams-poc-lib/main.c) file and the
[streams-poc-lib README](../sensor/streams-poc-lib/README.md) for more details.

When the streams-poc-lib test application has been
[build and flashed](../sensor/streams-poc-lib/README.md#build) and the
`idf.py` CLI is available, follow these steps to automatically initialize a
*streams-poc-lib test application sensor*:

* Depending on the state of the *Sensor* one of the following steps is needed to 
  properly manage the initialization status:
  * If the device has never been initialized: Move on to "Start the *IOTA Bridge*",
    to do a *Sensor* initialization.
  * If the device has already been initialized and has not been powered off
    ([why is this important?](#sensor-reinitialization---streams-poc-lib-test-application))
    you can just move on to "Start the *IOTA Bridge*", to do a *Sensor* **re**initialization.
  * If the device has already been initialized and has been powered off thereafter,
    you need to erase the flash of the *Sensor* device to do an initialization
    ([why is this needed?](#sensor-reinitialization---streams-poc-lib-test-application)).<br>
    You can run `idf.py erase-flash` and flash the
    *streams-poc-lib test application* again as been described in the
    [streams-poc-lib README](../sensor/streams-poc-lib/README.md#build).<br>
    You also need to delete the
    [*IOTA Bridge* SQLite database file](../iota-bridge/README.md#caching-of-lorawan-deveuis-and-streams-channel-meta-data)
    `iota-bridge.sqlite3` in the [workspace folder](#test-workspace).<br>
    [Here](../sensor/streams-poc-lib/README.md#sensor-initialization-vs-reinitialization)
    you can find out why this is needed.
* Start the *IOTA Bridge*
  ```bash
      > ./iota-bridge -l "192.168.47.11:50000"
      [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
      Listening on http://192.168.47.11:50000
  ```  
* Start the *streams-poc-lib* test application to listen for remote commands<br>
  The *streams-poc-lib* test application will start immediately after the boot sequence
  of the *Sensor* device. If you are using a USB interface for power supply and serial
  communication, this means the *Sensor* application will start several seconds
  after you have plugged in the USB cable.<br>
  To review the boot process and application start, you should **prepare** the
  IDF log monitoring tool in an additional shell in the root folder of the *streams-poc-lib*
  ([/sensor/streams-poc-lib](../sensor/streams-poc-lib)).
  To **prepare** means that you just type, but don't enter the last statement of the
  following commands. After preparing the log monitoring tool you power on the *Sensor* device
  and then you press enter:
  ```bash
      > cd ~/path-to-my-susee-streams-poc-repository/sensor/streams-poc-lib
      > get_idf
      > idf.py monitor                    # just type it - press enter after device power on
  ```
* Run the *Management Console* with the following options
  In an additional shell<br>
  ```bash
  > ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
  ```

The *Management Console* then will perform all the initialization steps fully automatically.
See the [CLI help for the `--init-sensor` option](../management-console/README.md#automatic-sensor-initialization)
of the *Management Console* for further details.

If you you want to test a *Sensor* **re**initialization, DO NOT power of the device and
process the above described test steps again.

#### Automatic Sensor Initialization - streams-poc-lib test application with AppServer Connector

In case the `SENSOR_MANAGER_CONNECTION_TYPE` in the test application
[main.c file](../sensor/streams-poc-lib/main/main.c) has been set to
`SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`,
the [*AppServer Connector Mockup Tool*](../app-srv-connector-mock) needs to be run in an
additional shell to perform the test steps described above:
```bash
    > ./app-srv-connector-mock -l 192.168.47.11:50001`
```  
The *AppServer Connector Mockup Tool* communicates with the *IOTA Bridge* via localhost therefore
the *IOTA Bridge* needs to be started without any command line arguments:
```bash
    > ./iota-bridge
    [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    Listening on http://127.0.0.1:50000
``` 
The *Management Console* also needs to access the *IOTA Bridge* via localhost:
```bash
    > ./management-console --init-sensor --iota-bridge-url "http://127.0.0.1:50000"
``` 

#### Automatic Sensor Initialization - x86/PC

Follow these steps to automatically initialize an *x86/PC Sensor*.
If you want to test a
[Sensor reinitialization](#sensor-reinitialization---x86pc)
later on we recommend to use the
[--dev-eui](../sensor/README.md#static-deveui)
argument of the *Sensor* CLI to specify a static DevEUI.

* Make sure that the *Streams* channel is not already initialized<br>
  If the *Sensor* has already been initialized,
  delete the `wallet-sensor.txt` and `user-state-sensor.bin`
  files in the [workspace folder](#test-workspace).<br>
* Start the *IOTA Bridge*<br>
  The *IOTA Bridge* needs to listen on localhost which is the default setting:
  ```bash
  > ./iota-bridge
  [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
  Listening on http://127.0.0.1:50000
  ``` 
* Start the *x86/PC Sensor* to listen for remote commands<br>
  In an additional shell:
  ```bash
  > ./sensor --act-as-remote-controlled-sensor
  ``` 
  if you want to test a
  [Sensor reinitialization](#sensor-reinitialization---x86pc)
  later on:
  ```bash
  > ./sensor --act-as-remote-controlled-sensor --dev-eui=12345678
  ```
* Run the *Management Console*<br>
  In an additional shell:
  ```bash
  > ./management-console --init-sensor --iota-bridge-url "http://127.0.0.1:50000"
  ```

The *Management Console* then will perform all the initialization steps fully automatically.

#### Automatic Sensor Initialization - ESP32 Sensor

__MAINTAINANCE MODE FOR ESP32 SENSOR APPLICATION__<br>
Please note that the *ESP32 Sensor* application is only maintained but its
functionality will not be extended to support tests for the latest versions
of the SUSEE application protocol. For tests on ESP32 devices the
[*streams-poc-lib test application*](#automatic-sensor-initialization---streams-poc-lib-test-application)
is the recommended application.

Similar to the *streams-poc-lib test application*, the *ESP32 Sensor* needs to be build
and flashed on the *Sensor* device. This is described
[here](../sensor/main-rust-esp-rs/README.md#prerequisites).

Please note that the [environment variables](../sensor/main-rust-esp-rs/README.md#build)
`SENSOR_MAIN_POC_WIFI_SSID`, `..._WIFI_PASS`
and `..._IOTA_BRIDGE_URL` need to be set correctly, equivalent to the precompiler
macros used in the *streams-poc-lib test application*
(see [above](#automatic-sensor-initialization---streams-poc-lib-test-application)).

Follow these steps to automatically initialize an *ESP32 Sensor*:

* Make sure that the *Streams* channel is not already initialized<br>
  If the *Sensor* has already been initialized, there are two options
  to set its state back to an uninitialized state:<br> 
  * The easiest way is to use the *Espressif IDF SDK* tool `idf.py` to erase the flash
    and to flash the *ESP32 Sensor* to the device again:
    ```bash
        > get_idf
        > idf.py erase_flash
        > cargo espflash --monitor --partition-table="partitions.csv" --release
    ```      
  * Remotely execute the ` --clear-client-state` functionality
    of the *ESP32 Sensor* app.
    Use the `--act-as-remote-control` argument of the *x86/PC Sensor* to remote-control
    the *ESP32 Sensor* as been described
    [here](../sensor/README.md#remote-control-cli-commands).<br>
* Start the *IOTA Bridge*<br>
  The *ESP32 Sensor* will communicate with the *IOTA Bridge* directly via WiFi
  so that the *IOTA Bridge* needs to listen on the external ip address:
  ```bash
  > ./iota-bridge -l "192.168.47.11:50000"
  ``` 
* Start the *ESP32 Sensor* application to listen for remote commands<br>
  In an additional shell in the folder `/sensor/main-rust-esp-rs`:
  ```bash
  > cargo espmonitor --chip=esp32c3 /dev/ttyYOURPORT`
  ```
* Run the *Management Console* with the following options
  In an additional shell:
  ```bash
  > ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"
  ```  

The *Management Console* then will perform all the initialization steps fully automatically.

### Sensor Reinitialization

As been described in the
[Sensor initialization section above](#sensor-initialization) the tests described
above perform a *Sensor* initialization that differs from a *Sensor* reinitialization.

During a *Sensor* reinitialization the DevEUI and the
Seed (secret key to derive private-public key pairs) of the Sensor is maintained, and
the *initialization count* of the *Sensor* is incremented. This is described in more detail
in the [Sensor README](../sensor/README.md#initialization-count).

#### Sensor Reinitialization - streams-poc-lib test application

As been described in the
[streams-poc-lib README](../sensor/streams-poc-lib/README.md#sensor-initialization-vs-reinitialization)
we recommend to reinitialize a *streams-poc-lib test application Sensor* after it has been
initialized once.

To test a *Sensor* reinitialization with the *streams-poc-lib test application*
you just need to follow the initialization steps described
[above](#automatic-sensor-initialization---streams-poc-lib-test-application)
directly after a *Sensor* initialization has been finished, without powering off
the device and without erasing the device flash storage.

In the [uninitialized-mode](../sensor/streams-poc-lib/README.md#uninitialized-mode)
the test-application will poll commands until the device is powered off.
After a *Sensor* initialization followed by a device reboot, the *Sensor* will be in the
[initialized-mode](../sensor/streams-poc-lib/README.md#initialized-mode)
and will continuously send messages but will not poll any commands anymore.

Therefore testing a *Sensor* reinitialization, using the *streams-poc-lib
test application*, currently is only possible in the uninitialized-mode.

For later use in production, the SUSEE application protocol needs to be extended to start
command polling on demand during the
[sensor-processing](../README.md#sensor-processing) workflow.

#### Sensor Reinitialization - x86/PC

In contrast to the [Sensor initialization](#automatic-sensor-initialization---x86pc)
the `wallet-sensor.txt` and `user-state-sensor.bin` files are maintained.

To reuse the [mocked DevEUI](../sensor/README.md#mocked-deveuis) we need to
specify the DevEUI using the [--dev-eui](../sensor/README.md#static-deveui)
argument. There are two possible ways to do this:
* Use the `--dev-eui` *Sensor* CLI argument during the
  [initialization](#automatic-sensor-initialization---x86pc)
  of the *Sensor* (recommended). 
* Use a hex or text editor to open the `wallet-sensor.txt` file in the
  [workspace folder](#test-workspace) and copy the value of the randomly chosen
  DevEui. The DevEui is located at the back  of the file and is encoded as utf8
  string in decimal presentation (radix 10).

Follow these steps to automatically **re**initialize a *x86/PC Sensor*.

* Start the *IOTA Bridge*<br>
  The *IOTA Bridge* needs to listen on localhost which is the default setting:
```bash
    > ./iota-bridge
    [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    Listening on http://127.0.0.1:50000
``` 
* Start the *x86/PC Sensor* to listen for remote commands<br>
  In an additional shell:
  ```bash
  > ./sensor --act-as-remote-controlled-sensor --dev-eui=12345678
  ```
  Please replace `12345678` with the used DevEUI.
* Run the *Management Console*<br>
  In an additional shell:
  ```bash
  > ./management-console --init-sensor --iota-bridge-url "http://127.0.0.1:50000"
  ```

The *Management Console* then will perform all the initialization steps fully automatically.

### Manual Sensor Initialization

The recommended way to initialize a *Sensor* is the
[automatic initialization](#automatic-sensor-initialization).
The manual *Sensor* initialization described here may be usefull 
to have more insights into the initialization process.
 
The process uses the *Sensor* and *Management Console* CLI to process each
initialization step.

Depending on the *Sensor* app (x86/PC, ESP32 Sensor, streams-poc-lib test application) the steps
to initialize the sensor are different. In the following
we only describe the *Sensor* initialization for the *x86/PC Sensor* and the
[streams-poc-lib test application](#subscribe-the-sensor---streams-poc-lib-test-application).

#### Create the channel using the *Management Console*

In the [workspace](#test-workspace) folder:
```bash
    > ./management-console --create-channel
    
      [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
      seed_derivation_phrase: YXWVURQPONKJIHGDCBA9XWVUTQPONMJIHGFCBA9ZWVUTSPONMLIHGFEBA9ZYVUTSRONMLKHGFEDA9ZYXU
      [Management Console] A channel has been created with the following announcement link:
                           Announcement Link: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87
                                Tangle Index: 1ac42554c457897b8cc146665c6bed7ee7fe816f2a93c269517e7f3f350ce5d1
```
Please note that the logged [seed_derivation_phrase](../README.md#common-file-persistence)
does not compromise the seed used for the created *Streams* channel because it is used together with
the private seed of the *Management Console* to derive the *Streams* channel seed.

The logged Tangle Index can be used to find the announcement message via the
[IOTA Tangle Explorer](https://explorer.iota.org/mainnet).

#### Subscribe the *Sensor* - x86/PC version

To use a *Sensor* application we need to start the *IOTA Bridge* first.
In an additional shell in the [workspace](#test-workspace) start it like this:
```bash
    > ./iota-bridge
    
      [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
      Listening on http://127.0.0.1:50000
```

Now the subscription message can be created using the announcement link from the console log of the
*Management Console* above. Just enter the following in a command shell in the [workspace](#test-workspace) folder:
```bash
    > ./sensor --subscribe-announcement-link\
             "9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87"
    
      [StreamsTransportSocket.new_from_url()] Initializing instance with options:
      StreamsTransportSocketOptions:
           http_url: http://localhost:50000,
           dev_eui:  5702837152734510599,
           use_lorawan_rest:  false

      [StreamsTransportSocket.recv_message] Receiving message with 151 bytes tangle-message-payload:
      0000000104000000000000000000000000009d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760e0000019d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b7600edc02666d79c10bffa5e23a1b1e36144aed73c8ac68c8481ea5e3758ec26ccf9d4af2d9cce8287be5b3cfe1ab72b57df725cbd7477c883511a55e5b6f3d3800c
      
      [StreamsTransportSocket.send_message] Sending message with 279 bytes tangle-message-payload:
      000050010400000001349d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000b95d1456eac7595be498fa870000000000000000001d1f2ff9b9a87ae85e40c888a216ac7e92cb4032d37843d88ba71888f051c4440e000001b95d1456eac7595be498fa872b898b350029cccc87fd63abe9bf9740cfe24ba210e3b4ac7f3d828a707a75035b7362ce15b7580e3c21b128da06df7e1198dff750aa9edc8b83c25b8b8b764f8664ac099fa80633508f4a1370b1061adda31da493a1954df75ea0bd1c0fc6164d0ab357905083be689cf2acf5394aa7db6d9f9a24df4f51c4cc6175f962bdad7e747e0bc32d2907887ede24d36d19839690a71305e300f5b632fdc3460f210c
      
      [Sensor] A subscription with the following details has been created:
                   Subscription Link:     9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:fd0bb1141e0e38cceb386414
                        Tangle Index:     f77553a929bfe18dabe1c15180bfb9d562c0c551c0cc4e7636e6bfaa89cfc1b9
                   Subscriber public key: 1d1f2ff9b9a87ae85e40c888a216ac7e92cb4032d37843d88ba71888f051c444
                   Initialization count:  0
```

The *IOTA-Bridge* also logs every data package that is transferred. Regarding absolute length of transferred binary packages
only take the *IOTA-Bridge* log into account as these are the correct package sizes. *Sensor* and *Management-Console* only
log the sizes of the tangle-message-payload: 
```bash
[IOTA Bridge] Handling request /message?addr=9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87

-----------------------------------------------------------------
[IOTA-Bridge - DispatchStreams] receive_message_from_address() - Received Message from tangle with absolut length of 255 bytes. Data:
@9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87[0000000104000000000000000000000000009d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760e0000019d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b7600edc02666d79c10bffa5e23a1b1e36144aed73c8ac68c8481ea5e3758ec26ccf9d4af2d9cce8287be5b3cfe1ab72b57df725cbd7477c883511a55e5b6f3d3800c]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000

-----------------------------------------------------------------
[IOTA Bridge] Handling request /message/send

-----------------------------------------------------------------
[IOTA-Bridge - DispatchStreams] send_message() - Incoming Message to attach to tangle with absolut length of 383 bytes. Data:
@9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:fd0bb1141e0e38cceb386414[000050010400000001349d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000b95d1456eac7595be498fa870000000000000000001d1f2ff9b9a87ae85e40c888a216ac7e92cb4032d37843d88ba71888f051c4440e000001b95d1456eac7595be498fa872b898b350029cccc87fd63abe9bf9740cfe24ba210e3b4ac7f3d828a707a75035b7362ce15b7580e3c21b128da06df7e1198dff750aa9edc8b83c25b8b8b764f8664ac099fa80633508f4a1370b1061adda31da493a1954df75ea0bd1c0fc6164d0ab357905083be689cf2acf5394aa7db6d9f9a24df4f51c4cc6175f962bdad7e747e0bc32d2907887ede24d36d19839690a71305e300f5b632fdc3460f210c]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
```

The subscription link and public key then must be used with the management-console to accept the subscription
```bash
    > ./management-console\
            --subscription-link "9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:fd0bb1141e0e38cceb386414"\
            --subscription-pub-key "1d1f2ff9b9a87ae85e40c888a216ac7e92cb4032d37843d88ba71888f051c444"

      [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
      seed_derivation_phrase: YXWVURQPONKJIHGDCBA9XWVUTQPONMJIHGFCBA9ZWVUTSPONMLIHGFEBA9ZYVUTSRONMLKHGFEDA9ZYXU
      [Management Console] A keyload message has been created with the following keyload link:
                           Keyload link: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:14f52c1b44f91f3e8e5b9eb1
                           Tangle Index: c0afd0f66d410ccd4af5d1e2af6c6657d1e47eef8dc2eb3ff6dac65871a268e0
```

To finalize the subscription the keyload message link has to be registered by the *Sensor* because it is the root message
of the branch used by the *Sensor* to publish its messages.
```bash
    > ./sensor --register-keyload-msg "9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:14f52c1b44f91f3e8e5b9eb1"
      
      [StreamsTransportSocket.new_from_url()] Initializing instance with options:
      StreamsTransportSocketOptions:
           http_url: http://localhost:50000,
           dev_eui:  5702837152734510599,
           use_lorawan_rest:  false
      
      [SubscriberManager.register_keyload_msg()] - Replacing the old previous message link with new keyload message link
                                        Old previous message link: 00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
                                        Keyload message link: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:14f52c1b44f91f3e8e5b9eb1
      
      [Sensor] Messages will be send in the branch defined by the following keyload message:
                   Keyload  msg Link:     9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:14f52c1b44f91f3e8e5b9eb1
                        Tangle Index:     c0afd0f66d410ccd4af5d1e2af6c6657d1e47eef8dc2eb3ff6dac65871a268e0
                   Subscriber public key: 1d1f2ff9b9a87ae85e40c888a216ac7e92cb4032d37843d88ba71888f051c444
                   Initialization count:  0
```

#### Subscribe the *Sensor* - streams-poc-lib test application

If we run a *streams-poc-lib test application Sensor* we can initialize the *Sensor*
in the [uninitialized mode](../sensor/streams-poc-lib/README.md#uninitialized-mode)
combined with *x86/PC Sensor* used as
[remote control](../sensor/README.md#remote-control-cli-commands). 

In the following, we expect the *streams-poc-lib test application Sensor* to be compiled
with the `SENSOR_MANAGER_CONNECTION_TYPE` **not** been set to
`SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`. Have a look at the
[automatic-sensor-initialization](#automatic-sensor-initialization---streams-poc-lib-test-application)
section for more details.

The *IOTA-Bridge* must be started this way:
```bash
    > ./iota-bridge -l "192.168.47.11:50000"
    > 
    > [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > Listening on http://192.168.47.11:50000
```

Please replace the ip address used in this example with the ip address of the network interface of your computer.
Have a look at the
[Sensor Initialization](#sensor-initialization)
section for more details.

Before we can send the `subscribe-announcement-link` command to the *test application* you need to
connect the serial port of your ESP32 board to your computer. Given the *Sensor* is in the 
[uninitialized mode](../sensor/streams-poc-lib/README.md#uninitialized-mode) the
*ESP32 Sensor* will poll commands from the *IOTA-Bridge* every 5 seconds after it has been powered up.
 
To see the console log output of the *test application* you need to start a serial port monitor application like
`idf.py monitor` (or [cargo espmonitor](https://github.com/esp-rs/espmonitor) in case of the *ESP32 Sensor*).
```bash
    > get_idf
    > idf.py monitor
```

The console output will contain a lot of boot and WiFi initialization messages. The most important messages
are the following ones:
 ```bash
    I (1447) test_streams_poc_lib: [fn process_test] Streams channel for this sensor has not been initialized. Going to initialize the sensor
    I (1449) test_streams_poc_lib: [fn process_test] Calling prepare_lwip_socket_based_sensor_processing() to use start_sensor_manager() later on
    ...
    ...
    I (4982) esp_netif_handlers: sta ip: 192.168.0.100, mask: 255.255.255.0, gw: 192.168.0.254
    I (4983) test_streams_poc_lib: [fn wifi_init_event_handler] Got ip:192.168.0.100
    I (4989) test_streams_poc_lib: [fn wifi_init_sta] connected to wifi SSID:Susee Demo password:susee-rocks
    I (4999) test_streams_poc_lib: [fn prepare_lwip_socket_based_sensor_processing] Preparing netif and creating default event loop
    
    I (5011) test_streams_poc_lib: [fn init_sensor_via_callback_io] Starting sensor_manager using IOTA-Bridge: http://192.168.0.101:50000
    I (5024) streams_poc_lib: [fn start_sensor_manager()] Starting
    I (5031) sensor_lib::esp_rs::main: [fn print_heap_info] heap_caps_get_free_size(MALLOC_CAP_8BIT): 149036
    I (5041) sensor_lib::esp_rs::main: [fn process_main_esp_rs] Using callback functions to send and receive binary packages
    I (5638) HTTP_CLIENT: Body received in fetch header state, 0x3fcbc437, 7
    Received Command::NO_COMMAND    
    Fetching next command in 2 secs
    ...
 ```
Now we can the send the `subscribe-announcement-link` command to the *test application* using the
*x86/PC Sensor* app. The CLI command is almost the same as used in the
[Subscribe the *Sensor* x86/PC version](#subscribe-the-sensor---x86pc-version) section.
We only need to add the `--act-as-remote-control` and `--iota-bridge-url` command to use the *Sensor* app 
as remote control for the *ESP32 Sensor*:
 ```bash
     > ./sensor -c -b "http://192.168.47.11:50000" --subscribe-announcement-link\
              "9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87"

    [Sensor] Acting as remote sensor using http://192.168.0.101:50000 as iota-bridge url
    [Sensor] Sending subscribe_announcement_link command to remote sensor. announcement_link: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87
    Received Confirmation::NO_CONFIRMATION    
    [Sensor] Remote sensor confirmed Subscription: Subscription:
        subscription_link: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:5d4b48fa2045f727dea5e63f
        pup_key: 4a905c7963f9c9d3e6e98b7b5e210eefb8b2456bd3ae05bed12ec35f8e632b11
        initialization_cnt: 0
 ```

The whole communication between the *x86/PC Sensor* remote control and the 
*streams-poc-lib test application* can be reviewed in the *IOTA-Bridge* log:
 ```bash
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /command/subscribe_to_announcement
    
    [IOTA-Bridge - DispatchCommand] subscribe_to_announcement() - Received command SUBSCRIBE_TO_ANNOUNCEMENT_LINK.
    Binary length: 110
    Queue length: 1
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /lorawan-rest/binary_request?deveui=180796021399420
    
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '180796021399420' with 26 bytes length
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request
    IotaBridgeRequestParts:
                         method: GET
                         uri: /command/next
                         body length: 0
                    
    [IOTA-Bridge - DispatchCommand] fetch_next_command() - Returning command SUBSCRIBE_TO_ANNOUNCEMENT_LINK.
    Blob length: 110
    Queue length: 0
    [dispatch_lorawan_rest_request] Returning response for dev_eui '180796021399420'
    IotaBridgeResponseParts:
                         status: 200 OK
                         body length: 110
                    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /lorawan-rest/binary_request?deveui=180796021399420
    
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '180796021399420' with 132 bytes length
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request
    IotaBridgeRequestParts:
                         method: GET
                         uri: /message?addr=9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87
                         body length: 0
                    
    -----------------------------------------------------------------
    [IOTA-Bridge - DispatchStreams] receive_message_from_address() - Received Message from tangle with absolut length of 255 bytes. Data:
    @9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87[0000000104000000000000000000000000009d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760e0000019d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b7600edc02666d79c10bffa5e23a1b1e36144aed73c8ac68c8481ea5e3758ec26ccf9d4af2d9cce8287be5b3cfe1ab72b57df725cbd7477c883511a55e5b6f3d3800c]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    
    [dispatch_lorawan_rest_request] Returning response for dev_eui '180796021399420'
    IotaBridgeResponseParts:
                         status: 208 Already Reported
                         body length: 259
                    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /lorawan-rest/binary_request?deveui=180796021399420
    
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '180796021399420' with 333 bytes length
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request
    IotaBridgeRequestParts:
                         method: POST
                         uri: /message/compressed/send
                         body length: 296
                    
    -----------------------------------------------------------------
    [IOTA-Bridge - DispatchStreams] send_message() - Incoming Message to attach to tangle with absolut length of 383 bytes. Data:
    @9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:5d4b48fa2045f727dea5e63f[000050010400000001349d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000b95d1456eac7595be498fa870000000000000000004a905c7963f9c9d3e6e98b7b5e210eefb8b2456bd3ae05bed12ec35f8e632b110e000001b95d1456eac7595be498fa8768e92219a1281a10a52eecd9d2f10827cc696affc000e4e040c39878d166143d71e4fd53a309cbbcd55615929408879d3e4120f24275d350c7ef3c68d7d59f7c6858c918b8072daa7e737945220894ec5a40db12ebf204e8465cb95337096614ff1590dfb52eba0b7c7e72958e24ed49d841728a597a3f2c5bcbf9e7b04b91af5af7f660bf51502be7c3574a82c51b863de2b84482a799a8f92293b590089300]->00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000
    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /confirm/next
    
    [IOTA-Bridge - DispatchConfirm] fetch_next_confirmation() - No confirmation available. Returning Confirmation::NO_CONFIRMATION.
    
    [dispatch_lorawan_rest_request] Returning response for dev_eui '180796021399420'
    IotaBridgeResponseParts:
                         status: 200 OK
                         body length: 0
                    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /lorawan-rest/binary_request?deveui=180796021399420
    
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '180796021399420' with 213 bytes length
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request
    IotaBridgeRequestParts:
                         method: POST
                         uri: /confirm/subscription
                         body length: 179
                    
    [IOTA-Bridge - DispatchConfirm] subscription() - Received confirmation SUBSCRIPTION.
    Binary length: 179
    Queue length: 1
    [dispatch_lorawan_rest_request] Returning response for dev_eui '180796021399420'
    IotaBridgeResponseParts:
                         status: 200 OK
                         body length: 0
                    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /lorawan-rest/binary_request?deveui=180796021399420
    
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Incoming request for dev_eui '180796021399420' with 26 bytes length
    [IOTA-Bridge - DispatchLorawanRest] post_binary_request() - Request is valid DispatchLorawanRest request
    IotaBridgeRequestParts:
                         method: GET
                         uri: /command/next
                         body length: 0
                    
    [IOTA-Bridge - DispatchCommand] fetch_next_command() - No command available. Returning Command::NO_COMMAND.
    
    [dispatch_lorawan_rest_request] Returning response for dev_eui '180796021399420'
    IotaBridgeResponseParts:
                         status: 200 OK
                         body length: 1
                    
    -----------------------------------------------------------------
    [IOTA Bridge] Handling request /confirm/next
    
    [IOTA-Bridge - DispatchConfirm] fetch_next_confirmation() - Returning confirmation SUBSCRIPTION.
    Blob length: 179
    Queue length: 0
 ```
Please note that during the process the *Sensor* and the *IOTA Bridge* switched from
[uncompressed messages](../sensor/README.md#deveuis-and-compressed-streams-messages) to compressed
messages (search for `208 Already Reported` in the log output above).
After the *IOTA Bridge* responded the `208 Already Reported` status, the *Sensor* uses the
'message/compressed' endpoints of the *IOTA Bridge*.


Meanwhile the *streams-poc-lib test application* will output the following log information:
```bash
    I (113123) HTTP_CLIENT: Body received in fetch header state, 0x3fcbc9dd, 116
    I (113126) streams_tools::remote::command_processor: [fn run_command_fetch_loop] Starting process_command for command: SUBSCRIBE_TO_ANNOUNCEMENT_LINK.
    I (113139) sensor_lib::esp_rs::main: [fn print_heap_info] heap_caps_get_free_size(MALLOC_CAP_8BIT): 128176
    I (113144) streams_tools::remote::command_processor: [fn process_command]  processing SUBSCRIBE_ANNOUNCEMENT_LINK: 9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:b95d1456eac7595be498fa87
    I (113839) HTTP_CLIENT: Body received in fetch header state, 0x3fcc2e0b, 265
    I (113843) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback::request()] Received StatusCode::ALREADY_REPORTED (208)- Set use_compressed_msg = true
    I (113854) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback::request()] use_compressed_msg = 'true'
    I (113867) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback.recv_message_via_http] Received response with content length of 259
    I (113883) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback.recv_message] Receiving message with 151 bytes tangle-message-payload:
    0000000104000000000000000000000000009d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760e0000019d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b7600edc02666d79c10bffa5e23a1b1e36144aed73c8ac68c8481ea5e3758ec26ccf9d4af2d9cce8287be5b3cfe1ab72b57df725cbd7477c883511a55e5b6f3d3800c
    
    I (114071) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback.send_message] Sending message with 279 bytes tangle-message-payload:
    000050010400000001349d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000b95d1456eac7595be498fa870000000000000000004a905c7963f9c9d3e6e98b7b5e210eefb8b2456bd3ae05bed12ec35f8e632b110e000001b95d1456eac7595be498fa8768e92219a1281a10a52eecd9d2f10827cc696affc000e4e040c39878d166143d71e4fd53a309cbbcd55615929408879d3e4120f24275d350c7ef3c68d7d59f7c6858c918b8072daa7e737945220894ec5a40db12ebf204e8465cb95337096614ff1590dfb52eba0b7c7e72958e24ed49d841728a597a3f2c5bcbf9e7b04b91af5af7f660bf51502be7c3574a82c51b863de2b84482a799a8f92293b590089300
    
    I (118959) HTTP_CLIENT: Body received in fetch header state, 0x3fcc3437, 6
    I (118962) sensor_lib::esp_rs::streams_transport_via_buffer_cb: [StreamsTransportViaBufferCallback::request()] use_compressed_msg = 'true'
    [Sensor] New subscription:
             Subscription Link:     9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:5d4b48fa2045f727dea5e63f
                  Tangle Index:     64b410c8c2957caa984f6148d65db374d70cc90b7b01581bb123fa9aa5528396
             Subscriber public key: 4a905c7963f9c9d3e6e98b7b5e210eefb8b2456bd3ae05bed12ec35f8e632b11
             Initialization Count:  0

```

As with the x86/PC version of the *Sensor* app the console log of *IOTA-Bridge* and the *ESP32 Sensor* will
contain the length of transferred binary data (*IOTA-Bridge*) and the subscription link and subscriber
public key (*ESP32 Sensor*).  

The subscription link and public key then must be used with the management-console to accept the subscription as being
described in the x86/PC section above.

To finalize the subscription the keyload message link has to be registered by the *streams-poc-lib test application*.
Again the CLI command is almost the same as used in the
[Subscribe the *Sensor* x86/PC version](#subscribe-the-sensor---x86pc-version):
```bash
    > ./sensor -c -b "http://192.168.47.11:50000" --register-keyload-msg "9d507222fb77bb5980509d8224250932691cdfdac6e61b8048da6c7274f10b760000000000000000:dc4567247bbb6396057bfba9"
```

## Send messages using the *Sensor*

The following sections show how to send messages using a *streams-poc-lib test application* and *x86/PC Sensor*. 

### Send messages - streams-poc-lib test application

A *streams-poc-lib test application* in the [initialized mode](../sensor/streams-poc-lib/README.md#initialized-mode)
will start sending messages after the device has booted.

Before we start the *Sensor* we need to start the [*AppServer Connector Mockup Tool*](../app-srv-connector-mock)
in a command shell in the [workspace](#test-workspace) folder:
```bash
    > ./app-srv-connector-mock -l 192.168.47.11:50001
```

The *IOTA Bridge* is also started in an additional command shell in the [workspace](#test-workspace) folder.
The *AppServer Connector Mockup Tool* communicates with the *IOTA Bridge* via localhost therefore
the *IOTA Bridge* needs to be started without any command line arguments:
```bash
    > ./iota-bridge
```

Now we are ready to boot the *Sensor* device. To view the log output of the *test application* start
the monitoring tool in a command shell in the [workspace](#test-workspace) folder right after the
device has powered on:
```bash
    > get_idf
    > idf.py monitor
```

After successfully connecting to the WiFi the Sensor starts to send messages every 5 seconds.
 
### Send messages - x86/PC Sensor

Before we can send messages using the *x86/PC Sensor* we need to start the *IOTA Bridge*:
```bash
    > ./iota-bridge
```

The folder [test/payloads](./payloads) contains several message files that can be
send like this. In this example we assume that the [workspace](#test-workspace) folder
is the `target/release` folder:
```bash
    > ./sensor --file-to-send "../../test/payloads/meter_reading_1_compact.json"

    [StreamsTransportSocket.new_from_url()] Initializing instance with options:
    StreamsTransportSocketOptions:
         http_url: http://localhost:50000,
         dev_eui:  11032547256235370273,
         use_lorawan_rest:  false
    
    [Sensor] Message file '../../test/payloads/meter_reading_1_compact.json' contains 136 bytes payload
    
    Sending message file ../../test/payloads/meter_reading_1_compact.json
    
    [StreamsTransportSocket.recv_message] Receiving message with 298 bytes tangle-message-payload:
    00001001040000000134f9fe4cc2c7a410c7ef47fce620bcaa32ef138a1619df6e0c4dbbaaf3a198046500000000000000009b4a4b02b097a5aef5218a97000000000000000200f9fe4cc2c7a410c7ef47fce620bcaa32ef138a1619df6e0c4dbbaaf3a19804650e0000019b4a4b02b097a5aef5218a97ce445bd603732ec1e1501b5b1ec1a1a101010010a07eba358dfea330fca75c261b0fee382479c1f97be07a316e76002d76b03cb3ccd6b275cf295392f797b863c3577fa962718cd003747a23dc02244f32fe44359b87ff8e640c699e08418917139c98368cc99421a1766f792cdd22339d8bc4976c917ad1dbd44efeaecd5c4a299b4e14b1f7e1a933fca09ccdd296990df366e9672427e2a027c476169807bc45be969a5ec53fd48ddc261de5dccb4ff7110b
    
    [StreamsTransportSocket.send_message] Sending message with 354 bytes tangle-message-payload:
    00003001040000000134f9fe4cc2c7a410c7ef47fce620bcaa32ef138a1619df6e0c4dbbaaf3a19804650000000000000000a440b4921e414c3adb85f7290000000000000003008e3f72ebfa3898603a24612ba76c73b2f085e48b3faa664f48174d732fac87990e000001a440b4921e414c3adb85f7298e3f72ebfa3898603a24612ba76c73b2f085e48b3faa664f48174d732fac879900770b32be3212111caeedd779c3d0d324fabf44a5cf49ac08af86b00461d086dec3776db9a13e95e10184a00ac2094501eba08e426518d099fb40d290ad6753b1a6fd523db74842fd6901a78eda8d3b0a5e3faa731210f3084a74232157be73b4f8ffef7440025b6fd1b42aba2cc9b3a2fc7c521ca0b967cb07f5f747482e9eb8ef739356b9f10c53ab136562f024124b043f61a3693449a53f1de4300eb58c2be8cd9dcd8f9e383cec785fabe4b7134eaca3f892ec2f3292887bd1dfd06406ad0da927b3ab1c39236405
    
    Previous message address now is f9fe4cc2c7a410c7ef47fce620bcaa32ef138a1619df6e0c4dbbaaf3a19804650000000000000000:3d710a402b4b27db4ace90a7
    
    Sending Message again in 5 secs
```

## View Sensor messages using the *Message Explorer*

Messages that have been send by *Sensors* can be explored using the *Message Explorer* which can
be started using the
[--run-explorer-api-server](../management-console/README.md#run-message-explorer)
argument of the *Management-Console* CLI:

```bash
    > ./management-console --run-explorer-api-server
      [Management Console] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
      2023-06-26T11:42:49.840054Z  INFO listening on 127.0.0.1:8080
```

The REST API of the *Message Explorer* can be tested using the swagger-ui which is provided by the
*Message Explorer* also, and can be opened using the following link: http://127.0.0.1:8080/swagger-ui

<img src="message-explorer-swagger-ui-screenshot.png" alt="Swagger UI of the Message Explorer" width="600"/>

Using the `Try it out` buttons of the swagger-ui, you can list messages of a specific Sensor.
Please note that a *Sensor* is called *Node* here (used in the sense of LoRaWAN Node).

<img src="swagger-ui-try-it-out.png" alt="Try-it-out Button of the Swagger UI" width="600"/>

Click on the links provided below to open the endpoint specific swagger-ui form, which allows to edit
and execute API requests after you have pressed `Try it out`:

* List all existing *Nodes* using the
  [GET /nodes](http://127.0.0.1:8080/swagger-ui/#/nodes/nodes_index) endpoint. After pressing the `Try it out`
  and `Execute` buttons, the *Message Explorer* will respond to the request with a list of all existing *Nodes*
  resp. *Sensors*.
  
* Copy the channel-id of the *Sensor* of interest from the *Node* result list.
  
* List all messages of the *Node* using the
  [GET /messages](http://127.0.0.1:8080/swagger-ui/#/messages/messages_index) endpoint.
  You need to paste the copied channel-id into the `channel_id` form field before you can execute this
  request.

You can also use the *Message Explorer* to set the `name` and `external_id` field of a specific *Node*.
This can be done using the [PUT nodes/{channel_id}](http://127.0.0.1:8080/swagger-ui/#/nodes/nodes_put)
endpoint.
