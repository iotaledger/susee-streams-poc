# Sensor Resources

This folder contains several *Sensor* specific projects for x86/PC and ESP32-C3
platforms. Here is an overview about the contained sub folders:

* [main-rust](./main-rust)<br>
  A *Sensor* application for x86/PC written in Rust
* [main-rust-esp-rs](./main-rust-esp-rs)<br>
  A *Sensor* application for ESP32-C3 written in Rust using
  [esp-rs/esp-idf-sys](https://github.com/esp-rs/esp-idf-sys) and the
  [Espressif IDF SDK](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/about.html)
* [sensor-lib](./sensor-lib)<br>
  A Rust library containing shared sources for all *Sensor* applications and the
  *streams-poc-lib*
* [streams-poc-lib](./streams-poc-lib)<br>
  A static library providing C bindings for all functions needed in the SUSEE-Module
  and a test application written in C to test the library
* [main-streams-poc-lib-pio](./main-streams-poc-lib-pio)<br>
  A [PlatformIO](https://platformio.org/) project to build the *streams-poc-lib* test
  application. This project demonstrates how to integrate the *streams-poc-lib* files
  in a PlatformIO project.
  
## CLI of the Sensor Applications

There are three different Sensor applications for different purposes:

| Application  |  Platform | Purpose                       | Progr. Language |
|--------------|-----------|-------------------------------|-----------------|
| *streams-poc-lib* test application | ESP32  | Test the streams-poc-lib functionality using its C binding| C |
| *ESP32 Sensor*                     | ESP32  | Test the Rust code that builds the foundation of the streams-poc-lib without the limitations of a foreign function interface | Rust |
| *x86/PC Sensor*            | x86/PC | Test the *Sensor* Rust code on x86/PC platforms and mock *ESP32 Sensors* for integration tests on x86/PCs | Rust |

In addition to the common CLI options described in the
[CLI API section of the main README file](../README.md#common-cli-options-and-io-files)
all Sensor applications provide CLI commands to manage the Streams usage:
 
    -s, --subscribe-announcement-link <SUBSCRIBE_ANNOUNCEMENT_LINK>
            Subscribe to the channel via the specified announcement link.
            
    -r, --register-keyload-msg <KEYLOAD_MSG_LINK>
            Register the specified keyload message so that it can be used
            as root of the branch used to send messages later on.

    -f, --file-to-send [<FILE_TO_SEND>...]
            A message file that will be encrypted and send using the streams channel.
            The message will be resend every 10 Seconds in an endless loop.
            Use CTRL-C to stop processing.
            
    -p, --println-subscriber-status
            Print information about the current client status of the sensor.
            In streams the sensor is a subscriber so that this client status is called subscriber
            status.
            
        --clear-client-state
            Deletes the current client status of the sensor so that
            all subscriptions get lost and the sensor can be used to subscribe to a new Streams
            channel.
            TODO: In future versions the seed will also be replaced by a new generated seed.
            TODO: -----------------------------
                  --------  WARNING  ---------- Currently there is no confirmation cli dialog
                  -----------------------------       use this option carefully!
                              
### Remote Control CLI commands
As all Sensor applications running on ESP32 do not provide an interactive terminal, the 
*x86/PC Sensor* can be used to remote control the ESP32
applications. The x86/PC Sensor provides following CLI commands to manage the
remote control functionality:

    -t, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            Default value is http://localhost:50000
            
            Example: iota-bridge-url="http://192.168.47.11:50000"

    -c, --act-as-remote-control
            Use this argument to remotely control a running sensor application on
            an embedded device. For example this
            
              > ./sensor --subscribe-announcement-link "c67551dade.....6daff2"\
                         --act-as-remote-control
            
            will make the remote sensor subscribe the channel via the specified
            announcement-link. This sensor app instance communicates with the remote sensor
            app via the iota-bridge application. Please make sure that both sensor
            app instances have a working connection to the running iota-bridge.
            
            If sensor and iota-bridge run on the same machine they can communicate over the
            loopback IP address (localhost). This is not possible in case the sensor runs on an
            external device (embedded MCU). In this case the iota-bridge needs to listen to
            the ip address of the network interface (the ip address of the device that runs
            the iota-bridge) so that the embedded sensor can access the iota-bridge.
            Therefore in case you are using 'act-as-remote-control' you will also need to use
            the 'iota-bridge' option to connect to the iota-bridge.

The *streams-poc-lib* test application can only bee remote controlled if the streams channel has
not already been initialized (further details can be found in the
[streams-poc-lib README](../sensor/streams-poc-lib/README.md)).

The x86/PC Sensor application can also be used to act as a remote controlled Sensor or let's say
to mock (or imitate) an ESP32 Sensor application.
This is especially usefull to test the *IOTA Bridge* and the *Management Console*
without the need to run ESP32 Hardware. The CLI command to mock an ESP32 Sensor is:

    -m, --act-as-remote-controlled-sensor
            Imitate a remote sensor resp. an ESP32-Sensor awaiting remote control commands.
            ESP32-Sensor here means the 'sensor/main-rust-esp-rs' application or the
            test app of the streams-poc-lib in an initial Streams channel state.
            
            This command is used to test the iota-bridge and the management-console application
            in case there are not enough ESP32 devices available. The sensor application will
            periodically fetch and process commands from the iota-bridge.
            
            If the iota-bridge runs on the same machine as this application, they can
            communicate over the loopback IP address (localhost). In case the sensor
            iota-bridge listens to the ip address of the network interface (the ip
            address of the device that runs the iota-bridge) e.g. because some ESP32
            sensors are also used, you need to use the CLI argument '--iota-bridge-url'
            to specify this ip address.

The `--act-as-remote-controlled-sensor` argument is especially useful to automatically initialize the x86/PC Sensor
in interaction with the 
[*Management Console* `--init-sensor` argument](../management-console/README.md#automatic-sensor-initialization).

### Exit after successful initialization

If you want to exit the *Sensor* application after the initialization has been finished you can use 
the `--exit-after-successful-initialization` argument:

    -e, --exit-after-successful-initialization
            If specified in combination with --act-as-remote-controlled-sensor the command poll loop
            will be stopped after a KEYLOAD_REGISTRATION confirmation has been send to confirm
            a successfully processed REGISTER_KEYLOAD_MESSAGE command.
            This argument is useful when the sensor app runs in automation scripts to allow the
            initialization of the Sensor and the Sensor app should exit after successful
            initialization.

### Use LoRaWAN Rest API

The x86/PC Sensor application can also be used to test the `lorawan-rest/binary_request` endpoint of the *IOTA Bridge*
application. This is done using the `--use-lorawan-rest-api` argument:

    -l, --use-lorawan-rest-api
            If used the Sensor application will not call iota-bridge API functions directly
            but will use its lorawan-rest API instead.
            This way the Sensor application imitates the behavior of an ESP32-Sensor connected
            via LoRaWAN and a package transceiver connected to the LoRaWAN application server
            that hands over binary packages to the iota-bridge.

### Static DevEUI

To test the [*Sensor Reinitialization* workflow](../README.md#sensor-reinitialization)
the mocked DevEUI of the *x86/PC Sensor* needs to be reused.
This can be achieved using the `--dev-eui` argument:

    -d, --dev-eui <DEV_EUI>
            Use the specified LoRaWAN DevEui instead of a random value.
            In case the sensor wallet file (wallet-sensor.txt) of the sensor has been deleted,
            the default behavior is to use a random value as new DevEui.
            The generated DevEui then is stored in the sensor wallet file later on so that the
            DevEui is persisted for later use.
            Using this argument the DevEui can be pre defined to have a static DevEui for test
            purposes. This argument is ignored in case the DevEui has already been stored in the
            sensor wallet file.
            
            Example: --dev-eu=12345678


## DevEUIs and Compressed Streams Messages

#### Compressed Streams Messages
To reduce the LoRaWAN payload size, compressed streams messages can be used to communicate
between a *Sensor* and the *IOTA Bridge*.
Compressed messages do not contain Streams Channel IDs and other data that can be restored by the
*IOTA Bridge*.

**IMPORTANT NOTE:** To restore the omitted data of a compressed message, the *IOTA Bridge*
can process on the encrypted message because the omitted parts of the message are plain text
metadata. This means that using compressed messages has no impact on data privacy. 
It would not be even possible to decrypt messages in the *IOTA Bridge*,
because in the SUSEE project the encryption key never leaves the
*Sensor* for security reasons.

#### Sensor to Bridge Pairing
The usage of compressed messages is only possible after one or more normal streams messages have
been send using the *IOTA Bridge*. The *IOTA Bridge* then learns which Streams Channel ID is used
by which *Sensor* where the *Sensor* is identified by its 64 bit LoraWAN DevEUI.
Using LoraWAN, the DevEUI is available via the protocol automatically and does not need 
to be transferred as message payload.

The mapping of LoraWAN DevEUI to Streams Channel ID is stored in a 
[local SQLite3 database](../iota-bridge/README.md#caching-of-lorawan-deveuis-and-streams-channel-meta-data)
managed by the *IOTA Bridge*.

In case the *IOTA Bridge* added a *Sensor* to its mapping database, the response of the 
REST call that caused the new *Sensor* database entry will have a `208 - ALREADY_REPORTED` http status.
All *Sensor* applications recognise this http response status and will only use compressed
messages further on. The Sensor applications store the state whether to use compressed
messages or not in their local user-state serialization files.

The process decribed above is called *Sensor to Bridge Pairing*.

The following sequence diagram shows the *Sensor to Bridge Pairing* in more detail:
<img src="Initial Sensor to IOTA-Bridge pairing.png" alt="Initial Sensor to IOTA-Bridge pairing" width="800"/>

#### Sensor to Bridge RE-Pairing
In case a *Sensor* uses compressed messages and is connected to an *IOTA Bridge* that does not know its
DevEUI the *IOTA Bridge* can not transmit the *Sensors* request to the *IOTA Tangle*.
This can happen e.g. if different *IOTA Bridge* instances are used for the
[Sensor Initialization](../README.md#initialization) and for
[Sensor Processing](../README.md#sensor-processing).

In this situation *Sensor to Bridge RE-Pairing* allows to transmit the channel-id later on which
results in a minimum of additional LoRaWAN payload.
After the IOTA-Bridge receives the missing channel-id, the original compressed request
is processed and the resulting response is returned to the sensor.

When the *IOTA Bridge* detects that a compressed request can not be processed, the relevant data
of the request are stored in its
[local SQLite3 database](../iota-bridge/README.md#caching-of-lorawan-deveuis-and-streams-channel-meta-data).

The Sensor receives a response with status `422 Unprocessable Content` together with a request_key
(response body) that can be used to address the unprocessed request later on. 

Using the `message/retransmit` endpoint of the *IOTA Bridge* the *Sensor* can provide the request_key
and *Streams Channel ID* to the *IOTA Bridge*. The bridge then fetches the original request from the
database and processes the request.

The `message/retransmit` request of the *Sensor* is answered by the *IOTA Bridge* with the response
that results from the original request that has been addressed using the request_key.
The status of this response is `208 Already Reported` to make shure that the *Sensor* uses
compressed requests further on.
 

The following sequence diagram shows the *Sensor to Bridge RE-Pairing* in more detail:
<img src="Sensor to IOTA-Bridge RE-Pairing.png" alt="Sensor to IOTA-Bridge RE-Pairing" width="800"/>

#### Initialization Count
During the [*Sensor Reinitialization* workflow](../README.md#sensor-reinitialization) a *Sensor* subscribes
to a new *IOTA Streams Channel* while the *Sensor* DevEUI remains unchanged.

The *Initialization Count* property of the SUSEE http protocol allows *IOTA Bridges* to detect reinitialized
sensors and to update the cached *IOOTA Streams* channel-id to the new channel-id via the
`messages/transmit` endpoint.

The *Initialization Count* is stored in the *Sensor* wallet file and is incremented when the sensor
subscribes to a new *Streams* channel.

The *IOTA Bridge* stores the *Initialization Count* together with the channel-id in its
[local SQLite3 database](../iota-bridge/README.md#caching-of-lorawan-deveuis-and-streams-channel-meta-data)
to detect reinitialized sensors.

The *Initialization Count* is loged to the console by the *Sensor* applications and by the *Management Console*.
Sensors can only be reinitialized 255 times. If this limit is reached a warning is loged by the
*Sensor* and by the *Management Console*.

A design goal of the SUSEE application protocol has been, to only include the *Sensor* and the *IOTA Bridge*
in the '*Sensor to Bridge Pairing*' and the '*Sensor to Bridge RE-Pairing*' process and to avoid any needed
third party interaction e.g. a central management logic. Until the maximum number of *Initialization Counts* is reached no central
management logic is needed.

In case the maximum number of *Initialization Counts* is reached there are several possible solutions to proceed:
1. Allow a counter reset to zero. This includes the risk that *IOTA Bridges* with a very outdated cache will
   not detect a reinitialized *Sensor* which will lead to erroneous communication until another *IOTA Bridge*
   is used that detects the new *Streams* channel-id.
2. Use the [`lorawan-node` endpoint](../iota-bridge/README.md#lorawan-node-endpoints) of all *IOTA Bridge*
   instances to delete the specific *Sensor* from the cache.
   This is the safest option, but a central management logic is needed to send the *IOTA Bridge*
   `lorawan-node` requests.

#### Mocked DevEUIs

The *Sensor* applications provided in this repository do not interact with LoRaWAN themselves.
Therefore, the DevEUIs are mocked for test purposes.
The following list explains how each *Sensor* application mocks the LoRaWAN DevEUI
and how the DevEUI is transfered to the *IOTA Bridge*:

* **Streams POC Library Test Application**<br>
  The LoraWAN DevEUI is mocked using the base MAC address of the ESP32 MCU
  (EUI-48 - formerly known as MAC-48).
  Espressif provides a universally administered EUI-48 address (UAA) for each
  network interface controller (NIC) e.g. WIFI, BT, ethernet, and so on.<br>
  To be independent from the used NIC we mock the LoraWAN DevEUI using
  the base MAC address that is used
  [to generate all other NIC specific MAC addresses](https://docs.espressif.com/projects/esp-idf/en/v3.1.7/api-reference/system/base_mac_address.html).
  <br>
  To make sure the mocked LoraWAN DevEUI is received by the
  [AppServer Connector Mockup Tool](../app-srv-connector-mock)
  the DevEUI is prepended to the request data that are send via the socket connection.
  The *AppServer Connector Mockup Tool* application later reads the mocked DevEUI from the socket
  stream and uses it to access the iota-bridge `/lorawan-rest` endpoints.
  <br><br>
* **x86/PC Sensor**<br>
  The LoRaWAN DevEUI is mocked using a persistent random value. The random value is stored
  in the wallet file together with the Streams channel
  [plain text seed](../README.md#common-file-persistence).
  The *x86/PC Sensor* application does not use the 
  [AppServer Connector Mockup Tool](../app-srv-connector-mock)
  to access the `/lorawan-rest` endpoints of the *IOTA Bridge*. Instead, it uses the
  `/lorawan-rest` endpoints directly in case the `--use-lorawan-rest-api` CLI argument
  is used.
  <br><br>
* **ESP32 Sensor**<br>
  Currently the *ESP32 Sensor* does not use `/lorawan-rest` endpoints of the *IOTA Bridge*
  and therefore does not need a mocked LoRaWAN DevEUI.

