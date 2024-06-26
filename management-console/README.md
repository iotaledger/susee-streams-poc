# Management Console

The *Management Console* is used to create new Streams channels and to add Sensors (a.k.a. Streams subscribers)
to those channels.

Managing multiple channels is possible. The streams client states of the
Streams channels are stored in a local SQLite3 database file (client-states-database)
as been described in the 
[Common file persistence](../README.md#common-file-persistence)
section.

The *Management Console* can also be used to start the *Message Explorer*
web service. See the [Run Message Explorer](#run-message-explorer)
section and the "*View Sensor messages using the Message Explorer*"
[section in the test documentation](../test/README.md#view-sensor-messages-using-the-message-explorer)
for more details.

The *Message Explorer* service can be deployed on a
[SUSEE Node](../susee-node/README.md), if the relevant 
configuration section in the 
[docker compose.yml file](../docker/README.md#start-iota-bridge-and-message-explorer-as-public-available-service)
has been uncommented.

## Prerequisites and Build
Please have a look at the [Prerequisites](../README.md#build-prerequisites)
and [Build](../README.md#build) section of the main README of this repository.

## Management Console CLI

In addition to the common CLI options described in the
[CLI API section of the main README file](../README.md#common-cli-options)
the *Management Console* offers the following CLI arguments.

#### Connections to SUSEE-Node Services

The following CLI arguments can be used to configure the communication
with the *IOTA Bridge* and the *IOTA Node*.

If you are using a local *IOTA Bridge* instance with private tangle
(as been described
[here](../susee-node/README.md#private-tangle-for-development-purposes)),
you don't need to specify any of these arguments.

Otherwise use the following arguments:

* `--iota-bridge-url`<br>
  The *IOTA Bridge* instance used to send/receive
  remote commands/confirmations
  (used for [automatic sensor initialization](#automatic-sensor-initialization))
* `--node`<br>
  The domain name of the
  [SUSEE Node](../susee-node)
  providing the
  *IOTA Node* web API used to interact with the *IOTA Tangle*.


Here is the CLI help text for both arguments:

    -b, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            The default value will work together with the private tangle for development purposes
            and a local running iota-bridge using the default settings.
            See folder 'susee-node' for more details.
            
            If your local iota-bridge listens to an external ip address, you need to specify this
            address using the --iota-bridge-url argument.
            
            If you are using an IOTA-Bridge provided by an external host, you need to specify the
            domain or address using the --iota-bridge-url argument. For example use
            "http://iotabridge.peeros.de:50000" for the SUSEE-Node provided by peerOS.
            
            Default value is http://127.0.0.1:50000
            
            Example: --iota-bridge-url="http://192.168.47.11:50000"

    -n, --node <NODE_URL>
            The IP or domain name of the iota node to connect to.
            As you need to provide also a streams inx-collector service instance,
            set this value to the domain name or static ip address of the host system
            that runs the inx-collector and the Hornet node.
            See folder 'susee-node' for more details.
            
            The Hornet node and inx-collector API will be accessed using their
            standard ports (14265 and 9030) automatically.
            
            The default settings will connect to the private tangle that can be run
            for development purposes (see folder 'susee-node' for more details).
            
            Examples:
                --node="195.90.200.153"
                -n="example.com"
             [default: 127.0.0.1]

#### Streams Channel Management

Following arguments are useful to manually initialize a *Sensor* as been described in the
[Manual Sensor Initialization section in the test documentation](../test/README.md#manual-sensor-initialization).
Usually the [Automatic Sensor Initialization](#automatic-sensor-initialization)
is preferred over *Manual Sensor Initialization*, so you might want to start with the
*Automatic Sensor Initialization*.

Here is the CLI help text for the *Streams Channel Management* CLI arguments:

    -c, --create-channel
            Use this option to create (announce) a new Streams channel.
            The announcement link will be logged to the console.
            The ID and streams_client_state of the new Streams channel will
            be stored in in the client-states-database.

    -e, --dev-eui <DEV_EUI>
            The DevEUI of the sensor to act on.
            DevEUI means 'device extended unique identifier' and is a term
            from LoRaWAN communication. Any random value (number or string)
            uniquely identifying the sensor can be used as long as the sensor
            uses the same value.
             
    -p, --println-channel-status
            Print information about currently existing channels.
            Each sensor is a subscriber in a dedicated Streams channel. The management-console
            manages these channels and stores the channel state information in its
            'client-states-management-console.sqlite3' database file. Use this CLI option to print
            the relevant channel state information from the SQLite database to console.
            Use CLI argument '--channel-starts-with' to select the Streams channel you want to
            investigate.

    -s, --channel-starts-with <CHANNEL_STARTS_WITH>
            Specify the Streams channel when processing a management-console
            CLI command. As the Streams channels ID has 40 byte length and is
            not easy to handle manually you only need to specify the beginning
            of the channels ID so that it can be found in the client-states-database.
            If there are more than one channels that can be found by the provided search string
            the command will fail.
            
            Example:
            
                >   ./management-console --channel-starts-with=6f5aa6cb --println-channel-status

Please also note the following dependencies between CLI arguments:
* The argument `--dev-eui` is needed for the `--create-channel` and for for the
  [Subscribe *Sensors*](#subscribe-sensors)
  arguments also.
* The `--channel-starts-with` argument is only needed for the `--println-channel-status` argument.

#### Subscribe Sensors
Following CLI arguments are used to subscribe *Sensors* to an existing channel,
if you are doing a manually *Sensor* initialization as been described in the
[Manual Sensor Initialization section in the test documentation](../test/README.md#manual-sensor-initialization):

    -k, --subscription-pub-key <SUBSCRIPTION_PUB_KEY>
            Add a Sensor to a Streams channel.
            The CLI argument defines the public key of the sensor subscriber.
            The public key of the sensor is logged to its console by the Sensor when the
            --subscribe-announcement-link CLI command is used.
            For more details have a look at the --subscription-link argument

    -l, --subscription-link <SUBSCRIPTION_LINK>
            Add a Sensor to a Streams channel.
            The CLI argument defines the subscription message link for the sensor subscriber.
            The subscription message link is logged to its console by the Sensor when the
            --subscribe-announcement-link CLI command is used.
            As the subscription message link contains the Streams channel ID the correct
            streams client state is fetched automatically out of the client-states-database.

The SUBSCRIPTION_PUB_KEY and SUBSCRIPTION_LINK will be logged to the console by the *Sensor* app when the 
CLI command --subscribe-announcement-link of the *Sensor* app is used.
This applies to the x86/PC version of the *Sensor* app and to the *Streams POC Library* test application.
In case of the *Streams POC Library* test application
these properties are also logged to the console of the *Sensor* app that is used as *Sensor remote control*.

#### Automatic Sensor Initialization

Instead of manually creating a Streams chanel and subscribing a Sensor,
the whole process (called *Sensor Initialization*) can be done automatically
using the `--init-sensor` argument.

**IMPORTANT NOTE:** 
The *Management Console* will do a *DevEUI Handshake*
[see below](#-deveui-handshake)
to find a *Sensor*, ready for automatic initialization.
After having finished an `--init-sensor`
process, **wait at least 10 Minutes** until a next `init-sensor`
or `--init-multiple-sensors` session is started.
See the explanations at the end of the
[Multiple Parallel Automatic *Sensor* Initialization](#multiple-parallel-automatic-sensor-initialization)
section below to find out the reason for this.

    -i, --init-sensor
            Initialize the streams channel of a remote sensor.
            The whole channel initialization is done automatically following the process described
            below. Management-console and remote sensor are communicating via the IOTA-Bridge.
            If your Sensor communicates with your IOTA-Bridge via an external domain or via an
            external port of your local system, you will need to use the '--iota-bridge' option
            to connect the Management-Console to the correct IOTA-Bridge.
            
            Example:
            
              > ./management-console --init-sensor --iota-bridge-url="http://192.168.47.11:50000"
            
            Please make sure that the remote sensor and the management-console have a working
            connection to the running iota-bridge.
            
            Initialization Process
            ----------------------
            The process consists of the following steps that could also be run manually using
            the CLI of the management-console and the sensor/ESP32-Sensor application:
            
                    ------------------------------------------------------
                    | management-console | --create-channel              |
                    |--------------------|--------------------------------
                    | sensor             | --subscribe-announcement-link |
                    |--------------------|-------------------------------|
                    | management-console | --subscription-link           |
                    |                    | --subscription-pub-key        |
                    |--------------------|-------------------------------|
                    | sensor             | --register-keyload-msg        |
                    ---------------------|--------------------------------
            
            As these CLI arguments require the --dev-eui argument the Management-Console
            performs a DevEUI-Handshake to determine the dev-eui of any suitable Sensor
            before the initialization process starts. Contrary to the --init-multiple-sensors
            argument the --init-sensor argument will only initialize one single sensor.
            
            In the automated initialization process all CLI commands and the data that are written
            to console log by the applications are transported using Command and Confirmation
            packages that are defined in the binary_persist module of the streams-tools library.
            
            Here is an overview which Command and Confirmation packages are used for communication
            with the remote sensor via the IOTA-Bridge:
            
             * management-console: Search for a dev_eui to start an initialization process
                                            # Send to ANY sensor using the DevEuiHandshakeCmd
                                            # Command
            
             * sensor: Provide a dev_eui for initialization
               --> DevEUI                   # Send to management-console using the DevEuiHandshake
                                            # Confirmation
            
             * management-console: --create-channel
                --> Announcement Link       # Send to the sensor using the SubscribeToAnnouncement
                                            # Command
            
             * sensor: --subscribe-announcement-link
                --> Subscription Link       # Send to the management-console using
                --> Public Key              # the SubscribeToAnnouncement Confirmation
            
             * management-console: --subscription-link --subscription-pub-key
                --> Keyload Link            # Send to the sensor using the RegisterKeyloadMessage
                                            # Command
            
             * sensor: --register-keyload-msg
                                            # Successful keyload registration is acknowledged with
                                            # a KEYLOAD_REGISTRATION Confirmation

To allow fully automated channel initializations the SUSEE Streams POC applications and the streams-poc-lib
are using an own communication protocol consisting of `commands` and `confirmations` where a `confirmation`
always carries the relevant data resulting from a command executed by a remote sensor.

Alternatively, to see the log output of the *Streams POC Library* test application, you can use a serial port monitor like `idf.py monitor`
or [cargo espmonitor](https://github.com/esp-rs/espmonitor).

If you use the `--init-sensor` option, all relevant *Streams Channel* properties like announcement-link,
subscription_pub_key, ... are logged to the console of the *Management Console* application,
equivalent to the usage of the *Sensor* application when it's used as a *Sensor remote control*. 

#### Multiple Parallel Automatic Sensor Initialization

If you need to initialize multiple *Sensors*,
use the following CLI argument to automatically initialize
multiple *Sensors* in parallel.

**IMPORTANT NOTE:** After having finished an `--init-multiple-sensors`
process, **wait at least 10 Minutes** until a next
`--init-multiple-sensors` or `--init-sensor` session is started.
See the explanations at the end of this section to find out the reason for this.

Here is the CLI help text for the `--init-multiple-sensors` CLI argument:

    -m, --init-multiple-sensors
            Initialize the streams channel of multiple sensors in parallel.
            Initializes a Sensor like the --init-sensor argument does, but will do this for
            an arbitrary amount of Sensors in parallel while --init-sensor will only initialize
            one single Sensor.
            
            The initialization process allways starts with a DevEUI-Handshake. During this
            handshake the Management-Console asks any Sensor for its DevEUI. Any Sensor that
            responds to a DevEUI-Handshake will receive all needed Commands as been described
            for the --init-sensor argument where the Commands are addressed to the specific
            Sensor using its DevEUI.
            
            After a DevEUI-Handshake has been completed the initialization is processed in its
            own thread so that many Sensor initializations can be done in parallel.
            
            Meanwhile to the Sensor initializations, the Management Console will search for
            additional Sensors that reply to a DevEUI-Handshake in an endless loop. This means
            that you need to kill the Management Console process after your last Sensor has
            been successfully initialized. Otherwise the Management Console would run
            infinitely.
            
            Example:
            
                >   ./management-console --init-multiple-sensors \
                                         --iota-bridge-url="http://192.168.47.11:50000"

The *Management Console* will do a first *DevEUI Handshake*
[see below](#-deveui-handshake)
to find a first *Sensor*, ready for automatic initialization.

After the first *DevEUI Handshake* has been successfully finished,
the *Management Console* will start the automatic *Sensor* initialization
for the found *Sensor* as been described [above](#automatic-sensor-initialization).

After the automatic *Sensor* initialization has been started,
the *Management Console* will immediately create a new
*DevEUI Handshake* command to find another *Sensor*
ready for automatic initialization.

The automatic *Sensor* initialization processes are
processed concurrently (means in parallel).
Every time the *Management Console* finds another *Sensor*,
ready for initialization, a new initialization processes
is spawned.

This also means, the *IOTA Bridge* always has
an available *DevEUI Handshake* command for the *DevEUI* `ANY`
while a *Multiple Parallel Automatic Sensor Initialization*
is running. This is also true for the state of the *IOTA Bridge*
after an automatic *Sensor* initialization.

The *IOTA Bridge*
therefore deletes all commands and confirmations after a
maximum lifetime of 10 Minutes. Otherwise a future
*Multiple Parallel Automatic Sensor Initialization*
process would start with an outdated *DevEUI Handshake* command
and the initialization would fail.

##### DevEUI Handshake

At the beginning of the normal *Sensor* initialization (`--init-sensor`) and the
multiple *Sensor* initialization (`--init-multiple-sensors`) 
described [above](#multiple-parallel-automatic-sensor-initialization)
a *DevEUI Handshake* is done by the *Management Console* and by any
*Sensor*, ready to start an initialization.

The *DevEUI Handshake* is used by the *Management Console* to find out
the *DevEUI* of a *Sensor*, ready to start the automatic initialization process.

A *DevEUI Handshake* process contains the following steps:

* The *Management Console* creates a *DevEUI Handshake* command via the 
  [IOTA Bridge web API](../iota-bridge/README.md#commands-and-confirmations).
  The *DevEUI Handshake* command is created for the *DevEUI* `ANY`,
  which is used as url parameter in the *IOTA Bridge web API*.
  
* A *Sensor* ready to be initialized, fetches the *DevEUI Handshake* command
  from the *IOTA Bridge*. It uses the *DevEUI* `ANY` as url parameter to
  fetch the command via the *IOTA Bridge web API*.
  
* The *Sensor* sends a *DevEUI Handshake* confirmation containing its
  *DevEUI* to the *Management Console*.<br>
  Although the confirmation itself contains the real *DevEUI* of the
  *Sensor*, it uses the *DevEUI* `ANY` as url parameter when the
  confirmation is created via the *IOTA Bridge web API*.
  
* The *Management Console* fetches the *DevEUI Handshake* confirmation
  and proceeds the automatic *Sensor* initialization, using the *DevEUI*
  contained in the confirmation for future command/confirmation communication.
  The automatic *Sensor* initialization itself is described
  [here](#automatic-sensor-initialization).<br>
  The *Management Console* uses the *DevEUI* `ANY` as url parameter
  to fetch the *DevEUI Handshake* confirmation from the *IOTA Bridge*
  via its web API but uses the received real *Sensor* *DevEUI* for
  future communication.
  
The *DevEUI* `ANY` is only used during the *DevEUI Handshake* process.
During all other processes, the commands and confirmations are
fetched/created using the real *DevEUI* of the *Sensor* that has
been initially exchanged via the *DevEUI Handshake*.

#### Run Message Explorer

You can explore the streams channels of existing LoRaWAN nodes and the the messages that have been
send via these channels using the `--run-explorer-api-server` CLI argument of the
management console:

    -r, --run-explorer-api-server <LISTENER_ADDRESS>
            Run an http rest api server to explore sensor messages stored on the tangle
            Default value for LISTENER_ADDRESS is 127.0.0.1:8080.
            
            After the server has been started you may want to:
            * fetch an overview about available paths from http://127.0.0.1:8080
            * explore the swagger-ui open-api documentation via http://127.0.0.1:8080/swagger-ui
            
            Example:
            
                >   ./management-console --run-explorer-api-server
            
            Specify the listener address and port for the server like this:
            Example:
            
                >   ./management-console --run-explorer-api-server 192.168.47.11:7777
                      
Alternatively to the live swagger-ui open-api documentation, available when the server is started,
you can view the REST api documentation
in the [online Swagger Editor](https://editor-next.swagger.io/).
Use the editors '_open file_' function to open the
latest [message-explorer-openapi.json](./message-explorer-openapi.json) file.
The file needs to be located on your local machine to open it.

Have a look into the [test documentation](../test#view-sensor-messages-using-the-message-explorer)
to find out more about how to use the swagger-ui to list messages of a specific sensor.

#### Using different *Management Console* instances for initialization and *Message Explorer*

If you are using a different *Management Console* instance to initialize the *Sensor*
and to run the *Message Explorer* you need to copy
two files from the initialization system to the *Message Explorer* system.

For example expect the initialization of the *Sensors* has been done on your local
development system and the *Message Explorer* runs on a
[SUSEE Node](../susee-node).
To upload the files from the initialization system to the 
*SUSEE Node* follow these steps:

* Create a folder `management-console-data` In the home folder of the
  *SUSEE Node* admin user.
* The folder where the *Management Console* has been run to initialize
  the *Sensors* contains a subfolder `data/management-console`.<br>
  Upload following files from `data/management-console` to the  
  previously created `management-console-data` folder on the *SUSEE Node*:
  * client-states-management-console.sqlite3
  * wallet-management-console.txt
* In the admin home folder of the *SUSEE Node*:<br>
  `$ sudo chown 65532:65532 management-console-data/*`<br>
  `$ sudo cp -a management-console-data/* susee-poc/data/management-console`<br>
  We expect here, that the docker compose environment runs in the subfolder
  `susee-poc` in the admin home folder as been described
  [here](../docker#start-iota-bridge-and-message-explorer-as-public-available-service).<br>
  The -a (--archive) flag used with the cp command helps to preserve file
  permissions and ownership.
* In the `susee-poc` subfolder of the admin home folder:<br>
  `docker compose restart management-console`
  