# Management Console

The *Management Console* is used to create new Streams channels and to add Sensors (a.k.a. Streams subscribers)
to those channels. Management of multiple channels is possible. The user states of the
Streams channels are stored in a local SQLite3 database file (user-states-database).

## Prerequisites and Build
Please have a look at the [Prerequisites](../README.md#prerequisites)
and [Build](../README.md#build) section of the main README of this repository.

## Management Console CLI

In addition to the common CLI options described in the
[CLI API section of the main README file](../README.md#common-cli-options)
the *Management Console* offers the following CLI arguments.

#### Connections to SUSEE-Node Services

    -b, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            The default value will work together with the private tangle for development purposes
            and a local running iota-bridge using the default settings.
            See folder 'inx-collector' for more details.
            
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
            See folder 'inx-collector' for more details.
            
            The Hornet node and inx-collector API will be accessed using their
            standard ports (14265 and 9030) automatically.
            
            The default settings will connect to the private tangle that can be run
            for development purposes (see folder 'inx-collector' for more details).
            
            Examples:
                --node="195.90.200.153"
                -n="example.com"
             [default: 127.0.0.1]

#### Streams Channel Management

    -c, --create-channel
            Use this option to create (announce) a new Streams channel.
            The announcement link will be logged to the console.
            The ID and user_state of the new Streams channel will be stored in in the
            user-states-database.

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
            'user-states-management-console.sqlite3' database file. Use this CLI option to print
            the relevant channel state information from the SQLite database to console.
            Use CLI argument '--channel-starts-with' to select the Streams channel you want to
            investigate.

    -s, --channel-starts-with <CHANNEL_STARTS_WITH>
            Specify the Streams channel when processing a management-console
            CLI command. As the Streams channels ID has 40 byte length and is
            not easy to handle manually you only need to specify the beginning
            of the channels ID so that it can be found in the user-states-database.
            If there are more than one channels that can be found by the provided search string
            the command will fail.
            
            Example:
            
                >   ./management-console --channel-starts-with=6f5aa6cb --println-channel-status

The argument `--dev-eui` is needed for the `--create-channel` and for for the
[Subscribe *Sensors*](#subscribe-sensors)
arguments also.

The `--channel-starts-with` argument is only needed for the `--println-channel-status` argument.

#### Subscribe *Sensors*
Following CLI arguments are used to subscribe *Sensors* to an existing channel:

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
            user state is fetched automatically out of the user-states-database.

The SUBSCRIPTION_PUB_KEY and SUBSCRIPTION_LINK will be logged to the console by the *Sensor* app when the 
CLI command --subscribe-announcement-link of the *Sensor* app is used. This applies to the x86/PC version 
of the *Sensor* app and to the *ESP32 Sensor* application. In case of the *ESP32 Sensor*
these properties are also logged to the console of the *Sensor* app that is used as *Sensor remote control*.

#### Automatic *Sensor* Initialization

Instead of creating a Streams chanel and subscribing a Sensor manually
the whole process (called *Sensor* initialization) can be done automatically:

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

To allow fully automated channel initializations the SUSEE Streams POC applications and the streams-poc-lib
are using an own communication protocol consisting of `commands` and `confirmations` where a `confirmation`
always carries the relevant data resulting from a command executed by a remote sensor.

Alternatively to see the log output of the *ESP32 Sensor* app you can use a serial port monitor like `idf.py monitor`
or [cargo espmonitor](https://github.com/esp-rs/espmonitor).

If you use the `--init-sensor` option all relevant Streams channel properties like announcement-link,
subscription_pub_key, ... are logged to the console of the *Management Console* app equivalent to
the usage of the *Sensor* app when it's used as a *Sensor remote control*. 

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

