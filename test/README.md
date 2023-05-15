# Tests

This folder contains documentation and resources to facilitate manual and automated tests. 
The documentation includes step by step descriptions of all tasks needed to perform manual
tests.  

Here is an overview of the contained sub-folders:

* [scripts](./scripts)<br>
  Contains script files to perform automatic
  tests for the *SUSEE Streams POC* applications and libraries. Have a look into the
  [test scripts README](./scripts/README.md) for more details.
* [iotqa-bridge](./iotqa-bridge) <br>
  Contains several curl scripts for manual testing of the *IOTA Bridge* API endpoints
  that are dedicated for public use.
  Have a look into the [*IOTA Bridge* README](../iota-bridge/README.md) for more details.
* [payloads](./payloads)<br>
  Contains several message files that can be used to test the *Sensors* send
  functionality (--file-to-send` argument of the *Sensor* application CLI).
  See below for more details.
  
 
The *Sensor* test applications can be tested manually by using the CLI of the applications.
This is described in the <a href="#sensor-initialization">"Sensor Initialization"</a>
and <a href="#send-messages-using-the-sensor">"Send messages using the Sensor"</a> section below.

Please note that the tests provided here underlie
[several restrictions](../README.md#restrictions-of-the-provided-tests) that are described in
the main README.

## Test workspace
As all built applications are located in the `target/debug` or `target/release`
sub-folder of the *susee-streams-poc* repository root folder, the easiest way
to run the tests described below is, to use one
of these folders as *test workspace* for manual testing.

We recommend using the release build of the applications because the proof of work,
done in the *IOTA Bridge*, is very time-consuming otherwise.

In the following test description we presume that the working directory of the
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

* As described [above](#test-workspace) we recommend to use the
  `target/release` sub-folder as *test workspace*.
* Initialization vs. Reinitialization<br>
  In the tests described below we will do a *Sensor* initialization and therefore
  we will make sure that the filesystem used by the *Sensor* app does not contain
  *IOTA Streams* user state files. Have a look into the
  [Sensor README](../sensor/README.md#initialization-count)
  for more details.
* In case a SUSEE POC application is listening to an external ip the example ip address
  `192.168.47.11` is used in the tests described below.
  Please replace the ip address with the ip address of the network interface of your computer.
  You need also to make sure that the used port is opened in the firewall of your OS.
  After having startet the application (e.g. the *IOTA-Bridge*) you can use telnet from another
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
command shell that runs parallel to the other two programs.

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
in a separated section.

As the test application always uses WiFi to connect to the LAN you will also
need to define the STREAMS_POC_LIB_TEST_WIFI_SSID, STREAMS_POC_LIB_TEST_WIFI_PASS,
STREAMS_POC_LIB_TEST_IOTA_BRIDGE_URL and STREAMS_POC_LIB_TEST_APP_SRV_CONNECTOR_MOCK_ADDRESS
precompiler macros in the [../sensor/streams-poc-lib/main.c](./main/main.c) file.
Please have a look at the `Test CONFIG` section of the 
[../sensor/streams-poc-lib/main.c](./main/main.c) file and the
[streams-poc-lib README](../sensor/streams-poc-lib/README.md) for more details.

When the streams-poc-lib test application has been
[build and flahed](../sensor/streams-poc-lib/README.md#build) and the
`idf.py` CLI is available, follow these steps to automatically initialize a
*streams-poc-lib test application sensor*:

* Make sure that the *Streams* channel is not already initialized<br>
  If the *Sensor* has already been initialized you need to run `idf.py erase-flash` and to flash the
  *streams-poc-lib* test application again.
* Start the *IOTA Bridge*
```bash
    > ./iota-bridge -l "192.168.47.11:50000"
    > 
    > [IOTA Bridge] Using node 'https://chrysalis-nodes.iota.org' for tangle connection
    > Listening on http://192.168.47.11:50000
```  
* Start the *streams-poc-lib* test application to listen for remote commands:<br>
  The *streams-poc-lib* test application will start immediately after the boot sequence
  of the *Sensor* device. If you are using a USB interface for power supply and serial
  communication, this means the *Sensor* application will start several seconds
  after you have plugged in the USB cable.<br>
  To review the boot process and application start, you should **prepare** the
  IDF logging tool in an additional shell in the root folder of the *streams-poc-lib*
  (path relative to repository root: [/sensor/streams-poc-lib](../sensor/streams-poc-lib)).
  Here to **prepare** means that you just type but don't enter the last statement of the
  following commands. After preparing the logging tool you power on the *Sensor* device
  and then you press enter:
```bash
    > cd ~/path-to-my-susee-streams-poc-repository/sensor/streams-poc-lib
    > get_idf
    > idf.py monitor                    # just type it - press enter after device power on
```
* Run the *Management Console* with the following options
  In an additional shell:<br>
  `> ./management-console --init-sensor --iota-bridge-url "http://192.168.47.11:50000"`<br>


The *Management Console* then will perform all the initialization steps fully automatically.
See the [CLI help for the `--init-sensor` option](../management-console/README.md#automatic-sensor-initialization)
of the *Management Console* for further details.

#### Automatic Sensor Initialization - streams-poc-lib test application with AppServer Connector

To perform the test steps described above in case 
the `SENSOR_MANAGER_CONNECTION_TYPE` in the test application
[main.c file](../sensor/streams-poc-lib/main/main.c) has been set to
`SMCT_CALLBACK_VIA_APP_SRV_CONNECTOR_MOCK`,
the [*AppServer Connector Mockup Tool*](../app-srv-connector-mock) needs to be run in an
additional shell like this:
```bash
    > ./app-srv-connector-mock -l 192.168.47.11:50001`
```  
The *AppServer Connector Mockup Tool* communicates with the *IOTA Bridge* via localhost therefore
the *IOTA Bridge* needs to be started without any command line arguments:
```bash
    > ./iota-bridge
``` 
The *Management Console* also needs to access the *IOTA Bridge* via localhost:
```bash
    > ./management-console --init-sensor --iota-bridge-url "http://127.0.0.1:50000"
``` 

### Automatic Sensor Initialization - ESP32 Sensor

TODO

### Automatic Sensor Initialization - x86/PC

TODO

### Manual Sensor Initialization

TODO