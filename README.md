# SUSEE Streams POC

## About
This project contains five test applications providing command line interfaces (CLI) to evaluate the *IOTA Streams*
functionality that is used in the SUSEE project. Additionally, the static library *streams-poc-lib* provides C bindings
for the most relevant *Sensor* specific functionality for the SUSEE project.

Following test applications are contained. For more details regarding the general workflows, actors,
roles and technical components of the SUSEE project please see below in the 
<a href="#workflow-model">Workflow Model</a> section:

* [IOTA Bridge](iota-bridge)<br>
  * Needed by all Sensor applications to access the IOTA Tangle
    * Provides an http rest api used by the *Sensor* applications to access the tangle<br>
    * Attaches the *Streams* packages received from the *Sensor* applications to the tangle
  * Forwards remote control commands from the *x86/PC Sensor* or *Management Console* to the Sensor applications
  * Forwards command confirmations from Sensor applications to the *x86/PC Sensor* or *Management Console*
* [ESP32 Sensor](sensor/main-rust-esp-rs)<br>
  * Imitates the processes running in the smart meter (a.k.a. *Sensor*)
  * Runs on ESP32-C3 devices
  * Can be remote controlled by the *x86/PC Sensor*
* [streams-poc-lib](sensor/streams-poc-lib)<br>
  * Provides C bindings for most functionalities of the *ESP32 Sensor*
  * Can be used with Espressifs ESP-IDF build process for ESP32-C3 devices
  * Includes a test application written in C to test the library functionality using a WIFI socket instead of
    a LoRaWAN connection
  * Provides most features of the *ESP32 Sensor* via its library interface
* [AppServer Connector Mockup Tool](app-srv-connector-mock)<br>
  * Acts as *Application Server Connector* for the *streams-poc-lib* test application
  * Receives & sends binary packages from/to the streams-poc-lib test application via a socket
    connection and transmits these packages to the *IOTA-Bridge* via its `lorawan-rest` API functions.
* [x86/PC Sensor](sensor/main-rust)<br>
  * Runs on x86/PC
  * Used to send commands to the *ESP32 Sensor* or *streams-poc-lib* test app
  * Can also be used to imitate an *ESP32 Sensor* on x86/PC platforms including
    the possibility to be remote controlled
* [Management Console](management-console)<br>
  * [Admin](#roles)-Tool to process workflows needed for *Initialization* of the *Sensor* and the monitoring of *Sensor Processing*
  * Manages the *Add/Remove Subscriber* workflows
  * Manages multiple channels resp. *Sensors* using a local SQLite3 database
  * Provides a [Message Explorer](management-console#run-message-explorer) to explore the *Sensor* messages 

###### How is IOTA Streams used?
The *Streams* channel used for the SUSEE project generally can be described as follows:
* One single branch per *Sensor*
* The Sensor will be a subscriber and will be the only publishing actor in the single branch
* The energy provider will be the author of the *Streams* channel
* Additional stakeholders (e.g. home owner) could be added as reading subscribers to the single branch
* Handshake:
  * The *Sensor* initialization (initial handshake consisting of announcement/subscription/keyload) between
    *Sensor* and the channel author will be done before a *Sensor* is installed in a home, which means for
    the initial handshake the limitations of LoRaWAN don't apply
  * If anything changes in the single branch channel setup, e.g. the addition of a new reading subscriber,
    the *Sensor* will have to be able to receive new keyload information downstream via LoRaWAN 

## Prerequisites

### For x86/PC

To build the applications for x86/PC platforms, you need the following:
- Rust - Please use the [official install script from rust-lang.org](https://www.rust-lang.org/tools/install)
  to have an up to date rust compiler (rustc). Do not use install packages provided with you OS because your
  rustc could be too old to build this project.

- (Optional) An IDE that supports Rust autocompletion. We recommend [Visual Studio Code](https://code.visualstudio.com/Download) with the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) extension

We also recommend updating Rust to the [latest stable version](https://github.com/rust-lang/rustup.rs#keeping-rust-up-to-date):

```bash
rustup update stable
```

### For ESP32

Please follow the steps described in the ESP32 specific application projects:
* [ESP32 Sensor](sensor/main-rust-esp-rs#prerequisites)
* [streams-poc-lib](sensor/streams-poc-lib#prerequisites)
* [PlatformIO Example for streams-poc-lib](sensor/main-streams-poc-lib-pio#prerequisites-and-build)

## Build

### For x86/PC

Build as usual using `build` or `run` with or without `--release`.

In the workspace root folder:
```bash
cargo build
```

Every application has its own crate so you might want to build only one application like this:

In the workspace root folder:
```bash
cargo build --package management-console  # alternatively 'sensor' or "iota-bridge"
```
All built applications are located in the `target/debug` or `target/release` subfolders of 
the workspace root folder.

The *ESP32 Sensor* is not build if `cargo build` is started in the workspace root folder.
The next section describes how to build it.

### For ESP32

Please follow the steps described in the ESP32 specific application projects:
* [ESP32 Sensor](sensor/main-rust-esp-rs#build)
* [streams-poc-lib](sensor/streams-poc-lib#build)
* [PlatformIO Example for streams-poc-lib](sensor/main-streams-poc-lib-pio#prerequisites-and-build)

## CLI API and file persistence 

### Common CLI options

Using the --help option of all four x86/PC applications will show the app specific help text:
```bash
target/release/management-console --help # Use 'sensor', 'app-srv-connector-mock' or "iota-bridge" instead of 'management-console' for the other apps
```

*Management Console* and the *x86/PC Sensor* provide the following options.
*IOTA-Bridge* and *AppServer Connector Mockup Tool* are using the same options expect `--wallet-file`
as these applications do not need a wallet:

    -h, --help
            Print help information

    -V, --version
            Print version information

    -w, --wallet-file <WALLET_FILE_PATH_AND_NAME>
            Specifies the wallet file to use.
            Set this to path and name of the wallet file.
            If this option is not used:
            * A file 'wallet-<APPLICATION-NAME>.txt' is used if existing
            * If 'wallet-<APPLICATION-NAME>.txt' does not exist:
              A new seed is created and written into a new file
              'wallet-<APPLICATION-NAME>.txt'.


#### Application specific CLIs

Please have a look at the application specific README files:

* [Management Console CLI](management-console/README.md#management-console-cli)
* [CLI of the Sensor Applications](sensor/README.md#cli-of-the-sensor-applications)
* [IOTA-Bridge Console CLI](iota-bridge/README.md#iota-bridge-console-cli)
* [AppServer Connector Mockup Tool CLI](app-srv-connector-mock/README.md#lorawan-appserver-mockup-tool-cli)

### Common file persistence

The *Management Console* and the [*Sensor* applications](./sensor/) use the following files for persistence
* Wallet for the user seed<br><br>
  *x86/PC*<br>
  The applications are using a plain text wallet that stores the automatically generated seed in a text file.
  If option '--wallet-file' is not used, a default filename 'wallet-<APPLICATION-NAME>.txt' is used.
  If the file does not exist, a new seed is created and stored in a new wallet file. Otherwise the seed stored
  in the wallet file is used.<br>
  
  As the wallet file contains the plain text seed (not encrypted) make absolutely sure to<br>
  **DO NOT USE THIS WALLET FOR PRODUCTION PURPOSES**<br>
  Instead implement the [SimpleWallet trait](streams-tools/src/wallet/plain_text_wallet.rs)
  using a secure wallet library like [stronghold](https://github.com/iotaledger/stronghold.rs).
  <br><br>
  The *Management Console* uses the seed to derive seeds for each managed channel.
  The channel seed is derived from the main seed, stored in the wallet file, and a seed-derivation-phrase,
  stored in the local SQLite3 database file.<br>
  
  *ESP32 Sensor*<br>
  The text file used for the plain text wallet is stored using
  [VFS](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/storage/vfs.html?highlight=vfs)
  and [FAT paths](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/storage/fatfs.html).
  For production purposes the seed needs to be stored in
  [encrypted flash or NVM storage](https://docs.espressif.com/projects/esp-jumpstart/en/latest/security.html).

* User state<br>
  *x86/PC*<br>
  On application start the current user state is loaded from a file named 'user-state-[APPLICATION-NAME].bin'.
  On application exit the current user state is written into this file.
  <br><br>
  The *Management Console* stores the user states of all managed channels in the local SQLite3 database file.
  <br><br>
  *ESP32*<br>
  The *ESP32 Sensor* reads and persists its user state every time a command is received
  from the *IOTA-Bridge*. Like the wallet text file the state is persisted in a FAT partition located in the SPI
  flash memory of the ESP32 board. This way the user state is secured against power outages of the ESP32.

<br>  

**IOTA Bridge**<br>
The *IOTA Bridge* stores a map of LoraWAN DevEUIs and *Streams* channel IDs in a local SQLite3
database "iota-bridge.sqlite3". More details can be found in the 
[Compressed Streams Messages](sensor/README.md#deveuis-and-compressed-streams-messages)
section.

## Test

Automatic tests and examples for manually performed workflows are described in the [README
of the test folder](./test/). 

#### Restrictions of the provided tests

A LoRaWAN communication infrastructure for test purposes is often not available.
Therefore the current POC *Sensor* applications (*ESP32 Sensor*, *x86/PC Sensor* and the 
*streams-poc-lib* test application) are using WiFi to connect to the *IOTA Bridge*
or the *AppServer Connector Mockup Tool*. This implies that the provided test
applications can not simulate a real world system due to the different
communication channel behaviors.  

###### Regarding *Sensor Initialization*:
* As described <a href="#initialization">below</a> the *Sensor* connection speed for the
  *Sensor Initialization* can be assumed as *"normal online connection"*.
  A WiFi connection therefore provides typical communication channel behavior and the tests
  should be close to real world usage.
* A WiFi connection is similar to a wired SLIP (Serial Line Internet
  Protocol) connection that might be used for automated hardware tests.
  
###### Regarding *Sensor Processing*:
The LoRaWan connection used in a real world scenario is different from the WiFi connection used
for the POC tests:
  * LoRaWan is much slower resp. the package transfer time can be of seconds magnitude.
  * Large packages will be split and automatically rejoined by the SUSEE LoRaWAN communication
    software stack. Although this allows larger packet sizes than using plain LoRaWAN connections, the payload
    size should be as low as possible.<br>
    As a rule of thumb:
    * Smaller than 512 bytes
    * Better are packages smaller than 256 bytes
    * Ideally smaller than 128 bytes
  * The [LoRaWAN Duty Cycle is restricted](https://www.thethingsnetwork.org/docs/lorawan/duty-cycle/)
    to allow public and permissionless usage of the used radio channels.
    [The *Fair Use Policy* of *The Things Network*](https://www.thethingsnetwork.org/docs/lorawan/duty-cycle/#fair-use-policy)
    provides a simplified rule to well behave as a a user of these radio channels:
    * Uplink Airtime is limited to 30 seconds per day (24 hours) per *Sensor*
    * Downlink messages are limited to 10 messages per day (24 hours) per *Sensor*.
* *streams-poc-lib* test application:<br>
  The slower connection speed is handled by the LoRaWAN network, so that the payload data can be assumed
  to be available at the *LoRaWAN Application Server* in one binary package (BLOB) at an unknown time in
  the future.<br>
  As long as the time span between the *Sensor Send Process* and the finished *LoRaWAN Application Server Receive
  Process* are not relevant for the overall information process, the poorer LoRaWAN communication speed
  has no impact on the *IOTA Bridge* and *Management Console*, and on the *streams-poc-lib* functionality.

## Workflow Model

In the SUSEE project the *Sensor* lifecycle consists of the following workflows:
* [Sensor Initialization](#initialization)
* [Sensor Processing](#sensor-processing)
* [Add/Remove Subscriber](#addremove-subscriber)
* [Sensor Reinitialization](#sensor-reinitialization)

In the next sections these workflows, the roles of the participating actors
and the software used to fulfill the associated activities are described in more detail. As this README focuses on the technical
implementation using the *IOTA Streams* library, only those aspects that
have an impact on the way *IOTA Streams* is used are described.    

### Actors

* End Customer<br>
  Energy consumer that can also be an energy producer (a.k.a. Prosumer).
  Has a contract with an energy provider or metering point operator.
  Usually the *Sensor* (a.k.a. smart meter) is located at the facility
  (house, company site, ...) of the *End Customer*.
* Energy provider or metering point operator<br>
  The company that runs the smart meter at the facility of the *End Customer*
  and is responsible that the metering complies to all regulatory standards. 
* Sensor Manufacturer<br>
  Produces the Sensor hardware.

### Roles

Here are the roles of the *SUSEE Workflows* that are impacted by *IOTA Streams*:
* *Sensor*
  * Sends meter data messages as encrypted *IOTA Streams* packages via LoRaWAN
  * Is the only participant in an *IOTA Streams* channel
    that sends meter data, so it uses a communication channel that is dedicated to it
  * Stores the identity of the *End Customer*
  * Allows the *End Customer* to directly manage the participation in the data transfer
    (e.g. to activate or cancel the participation)
  * Sends and receives control data (*Commands* and *Confirmations*)
    to participate in the management of the used *IOTA Streams* channel (these
    control data are not transfered via the *IOTA Streams* channel) 
* *Read Only Participant*<br>
  Can read meter data messages from a *Sensor* specific *IOTA Streams* channel
  (this could be for example the *End Customer* where the *Sensor* is installed)
* *Admin*
  * Administrates the *IOTA Streams* channel for each *Sensor*
  * Inserts or removes *Sensors* from the administrative system and creates
    dedicated *IOTA Streams* channels for them
  * Inserts or removes *Read Only Participants* to/from the Sensor dedicated *IOTA Streams* channel

### Technical Components

The following technical components are needed for the SUSEE system to implement the workflows:

* *Sensor*
  * Behaves as been described for the *Sensor* role (see above)
  * Runs on an embedded MCU which can be the smart meter device itself, or a
    closely connected extension unit (e.g. the SUSEE Module)
* *Management Console*
  * Application used by the *Admin* role, providing all needed functionality
    (described for the *Admin* role) to manage the *IOTA Streams* channels
    of all *Sensors* of an *Energy provider or metering point operator*
  * Runs at the *Energy provider or metering point operator*
* *LoRaWAN Application Server*
  * Handles the LoRaWAN application layer payloads from and to the Sensor
  * Provides data received via LoRaWAN from a *Sensor*
  * Sends data to a *Sensor* via LoRaWAN
  * Part of the LoRaWAN infrastructure that is run by the
    *Energy provider or metering point operator*
  * End point of the LoRaWAN network infrastructure from application layer perspective
    resp. *IOTA Streams* usage perspective
* *IOTA Bridge*
  * Receives encrypted Streams packages and sends them into the *IOTA Tangle*
  * Receives Tangle messages from the *IOTA Tangle* and sends them to the Sensor
  * Transfers control data between the Sensor
    and the *Management-Console*
  * When used for [Sensor Processing](#sensor-processing):<br>
    * Closely connected to the *Application Server Connector*
      (same device or intranet or at least a very fast connection to it)
    * Receives meter data messages as encrypted *IOTA Streams* packages
      from the *Sensor* via the *LoRaWAN Application Server* and the
      *Application Server Connector* and sends *IOTA Tangle* messages in
      the opposite direction vice versa
    * Transfers control data in the same way as *IOTA Streams* packages and
      *IOTA Tangle* messages are transferred 
    * Offers a REST API for the *Application Server Connector*
      to manage the *IOTA Streams* package and *IOTA Tangle* message transfer
  * When used for [Sensor Initialization](#initialization):<br>
    * Runs at the actor which is responsible for the *Sensor Initialization*
      (energy provider, metering point operator, *Sensor* producer, ...)
    * Offers a REST API to receive *IOTA Streams* packages and send
      *IOTA Tangle* messages directly from resp. to the *Sensor* 
    * Offers a REST API to transfer control data directly from resp. to
      the *Sensor* and the *Management Console*
* *Application Server Connector*
  * A service connecting the *LoRaWAN Application Server* and the *IOTA Bridge*
  * Only used for the [Sensor Processing](#sensor-processing) workflow
  * Closely connected to the *LoRaWAN Application Server*
    (same device or intranet or at least a very fast connection to it)
  * Receives LoRaWAN payloads from the *LoRaWAN Application Server*
    (e.g. via MQTT) and provides the data to the *IOTA Bridge* using the
    REST API provided by the *IOTA Bridge* for this purpose
  * Sends LoRaWAN payloads to the *LoRaWAN Application Server* (e.g. via MQTT)
    that it has received by the *IOTA Bridge*
* [IOTA Tangle](https://wiki.iota.org/learn/about-iota/tangle/)
  * A distributed ledger consisting of a Directed Acyclic Graph (DAG)
    of [messages](https://wiki.iota.org/learn/about-iota/messages/)
    (a.k.a. blocks) that can bee accessed via *IOTA Nodes*

This code repository provides [console applications](#about) for several of the
components listed above (*Sensor*, *Management Console* and *IOTA Bridge*) to evaluate
the needed functionality in terms of technical feasibility.
Additionally, the [AppServer Connector Mockup Tool](app-srv-connector-mock) is provided to
act as an *Application Server Connector* for *Streams POC Library* tests.

Due to different target platforms and online access the roles resp. applications
underlay the following restrictions:

* *Sensor*
  * Connectivity
    * *Initialization*: Wifi or wired using peripherals (e.g. usb).
    * *Sensor Processing*: Wireless via LoRaWAN.
    * *Add/Remove Subscriber*: Wireless via LoRaWAN.
    * *Sensor Reinitialization*: Wifi or wired using peripherals (e.g. usb), eventually LoRaWAN .
  * Platform: Embedded low cost MCU.
    * Low processing capabilities.<br>
      Due to the low processing capabilities the *Sensor* does not send the streams packages
      to the tangle directly but sends the packages to the *IOTA Bridge*.
      This way it does not need to process the adaptive POW.<br>
      *Streams* packages coming from the tangle are also received via the *IOTA Bridge*.<br>
      This applies to all workflows (*Initialization*, *Sensor Processing*, *Add/Remove Subscriber*,
      *Sensor Reinitialization*) and for incoming and outgoing packages.
 * *Management Console*
    * Connectivity:<br>
      Fast (typical office online access).
    * Platform: X86/PC, standard PC hardware.
    * No hardware or performance restrictions for all workflows.
 * *IOTA Bridge*
   * Connectivity:<br>
     * For connections to the *Application Server Connector* and *Management Console*:<br>
       Fast (at least typical office online access).
     * For connections to the *Sensor*:<br>
       For *Sensor Processing* and *Add/Remove Subscriber* workflows, the *IOTA Bridge*
       and the *Sensor* communicate via the *LoRaWAN Application Server* and the
       *Application Server Connector*. Therefore the
       [LoRaWAN restrictions](#restrictions-of-the-provided-tests)
       regarding payload size, message count and message delays apply here.<br>
       As the communication with the *Application Server Connector* is fast,
       the time needed to transfer request- and response-packages is normal.
       Long lasting connections can be a problem for web servers in general,
       as the server needs to handle each request in a dedicated thread
       (e.g. GPRS mobile clients).
       Due to the fast connection to the *Application Server Connector*
       no measures have to be taken to handle slow clients
       (e.g. no nginx proxy needed).
       For the *IOTA Bridge* the connection timespan is mainly impacted
       by the communication with the *IOTA Tangle*.<a>
       During the *Initialization* workflow a fast Wifi or wired connection
       is given.
    * Platform: X86/PC, standard server or edge computing hardware depending
      on the chosen topology. 
    * In the current POC implementation the *IOTA-Bridge* forwards remote 
      control data (*Commands* and *Confirmations*) from the *Sensor*
      to the remote control (*x86/PC Sensor* or *Management Console*).
      In a later production system for the *Sensor Processing* workflow
      this service will probably be implemented as an independent service
      while for the *Initialization* workflow an integration
      of Tangle- and Command-Communication can be of advantage.

### Workflows

Following workflows will exist for each *IOTA Streams*
channel. Every *Sensor* uses its own exclusive channel:

#### Initialization

The *Sensor* initialization is the initial handshake between *Management Console*
and *Sensor*. It will be done before a *Sensor* is installed in an
*End Customers* facility and is controlled by the *Admin* role.

Regarding the *IOTA Streams* channel that is used to manage the communication
between all communication participants, the following *Streams* specific actions have to be performed:

| Module               | Streams Action                                                                 | Result        |
| -------------------- | ------------------------------------------------------------------------------ | ------------- |
| *Management Console* | Create a new *IOTA Streams* channel                                            | *Announcement Link* |
| *Sensor*             | Subscribe to the channel using the *Announcement Link*                         | *Subscription Link*, *Public Key* |
| *Management Console* | Add the Sensor to the channel using its *Subscription Link* and *Public Key*   | *Keyload Message* |
| *Sensor*             | Register the *Keyload Message* which specifies all participants of the channel | - |
<br>

Dataflow of the *Initialization Workflow*:

<img src="sensor-init-diagram.jpg" alt="Sensor Initialization Workflow" width="800"/>

Although in the above diagram the *IOTA Bridge* and *Management Console* are used on the same system,
both components could be connected using the internet. For example, the *IOTA Bridge* could be located
at the *Sensor Manufacturer* and the *Management Console* could be located and controlled by the
*Energy provider or metering point operator*.
  
#### Sensor Processing

Meter data are send by the *Sensor* to the *IOTA Tangle*. The *Sensor* is typically
located at the *End Customer*.

The meter data messages are created and encrypted into *IOTA Streams* packages by the *Sensor*.
The encrypted packages are send via LoRaWAN to the *LoRaWAN Application Server*.
An *Application Server Connector* receives the packages from the *LoRaWAN Application Server*
e.g. using MQTT. The *Application Server Connector*
transfers the packages to the *IOTA Bridge* using its `lorawan-rest` API endpoints.

The *Application Server Connector* and *Application Server Connector* are controlled by the
*Energy provider or metering point operator*.

Dataflow of the *Sensor Processing Workflow*:

<img src="sensor-processing-diagram.jpg" alt="Sensor Processing Workflow" width="800"/>

To allow the [reinitialization](#sensor-reinitialization) and
[Add/Remove Subscriber](#addremove-subscriber) workflows the SUSEE application
protocoll needs to provide the ability to switch between *Sensor Processing*,
*Add/Remove Subscriber* and *Sensor Reinitialization* workflows on demand.

#### Add/Remove Subscriber

Participants of the *Sensors* *IOTA Streams* channel are added or removed by the *Admin*.
For this purpose a new *Keyload Message* message is send from the *Management-Console* to the *Sensor*.

Contrary to the *Initialization* workflow, here LoRaWAN is also used for a back channel from the
*LoRaWAN Application Server* to the *Sensor*.

The dataflow matches the dataflow of the *Sensor Processing* workflow.

#### Sensor Reinitialization

The safest way to assure the best data privacy is to use different *IOTA Streams* channels.
In a situation where an already used *Sensor* hardware shall be reused in a different property
it may be suitable to create a new channel for the already initialized Sensor
(*Sensor Reinitialization*).

After a *Sensor Reinitialization* all evtl. needed *Read Only Participants* have to subscribe to the
new channel again. In case all channel participants shall have no access to *Sensor* messages of the
old *Sensor* (pre reinitialization messages) participants will be subscribed with new identities
(key pairs). In case a participant needs access to old and new messagges (e.g. *Admin* role) an
already existing identity could be reused.

Currently it is not clear if LoRaWAN can be used for *Sensor Reinitialization*.