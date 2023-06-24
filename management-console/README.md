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

#### Streams Channel Management

    -c, --create-channel
            Use this option to create (announce) a new Streams channel.
            The announcement link will be logged to the console.
            The ID and user_state of the new Streams channel will be stored in in the
            user-states-database.

    -n, --node <NODE_URL>
            The url of the iota node to connect to.
            Use 'https://chrysalis-nodes.iota.org' for the mainnet.
            
            As there are several testnets have a look at
                https://wiki.iota.org/learn/networks/testnets
            for alternative testnet urls.
            
            Example:
                The iota chrysalis devnet:
                https://api.lb-0.h.chrysalis-devnet.iota.cafe
             [default: https://chrysalis-nodes.iota.org]
             
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
            Therefore you also need to use the '--iota-bridge' option to connect the management-
            console to a running IOTA-Bridge.
            
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
            
            In the automated initialization process all CLI commands and the data that are written
            to console log by the applications are transported using Command and Confirmation
            packages that are defined in the binary_persist module of the streams-tools library.
            
            Here is an overview which Command and Confirmation packages are used for communication
            with the remote sensor via the IOTA-Bridge:
            
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

    -b, --iota-bridge-url <IOTA_BRIDGE_URL>
            The url of the iota-bridge to connect to.
            See --init-sensor for further information.
            Default value is http://localhost:50000
            
            Example: iota-bridge-url="http://192.168.47.11:50000"

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
            
            Specify the listener address and port for server like this:
            Example:
            
                >   ./management-console --run-explorer-api-server 192.168.47.11:7777
                      
Alternatively to the live swagger-ui open-api documentation, available when the server is started,
you can view the REST api documentation
in the [Swagger Editor](https://editor.swagger.io/?url=./message-explorer-openapi.json).