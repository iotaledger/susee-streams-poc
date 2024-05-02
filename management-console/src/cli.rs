use clap::{
    Arg
};

use streams_tools::STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL;

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli,
    cli_base::ArgMatchesAndOptions,
};

// TODO: Implement new CLI commands "--list-channels", "--user-states-database-path"
// Instead of "--user-states-database-path" a management-console.config file (toml) could be useful.

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub subscription_link: &'static str,
    pub subscription_pub_key: &'static str,
    pub create_channel: &'static str,
    pub init_sensor: &'static str,
    pub init_multiple_sensors: &'static str,
    pub iota_bridge_url: &'static str,
    pub dev_eui: &'static str,
    pub println_channel_status: &'static str,
    pub channel_starts_with: &'static str,
    pub run_explorer_api_server: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    subscription_link: "subscription-link",
    subscription_pub_key: "subscription-pub-key",
    create_channel: "create-channel",
    init_sensor: "init-sensor",
    init_multiple_sensors: "init-multiple-sensors",
    iota_bridge_url: "iota-bridge-url",
    dev_eui: "dev-eui",
    println_channel_status: "println-channel-status",
    channel_starts_with: "channel-starts-with",
    run_explorer_api_server: "run-explorer-api-server"
};

static SUBSCRIPTION_LINK_ABOUT: &str = "Add a Sensor to a Streams channel.
The CLI argument defines the subscription message link for the sensor subscriber.
The subscription message link is logged to its console by the Sensor when the
--subscribe-announcement-link CLI command is used.
As the subscription message link contains the Streams channel ID the correct
user state is fetched automatically out of the user-states-database.
";

static SUBSCRIPTION_PUB_KEY_ABOUT: &str = "Add a Sensor to a Streams channel.
The CLI argument defines the public key of the sensor subscriber.
The public key of the sensor is logged to its console by the Sensor when the
--subscribe-announcement-link CLI command is used.
For more details have a look at the --subscription-link argument
";

static CREATE_CHANNEL_ABOUT: &str = "Use this option to create (announce) a new Streams channel.
The announcement link will be logged to the console.
The ID and user_state of the new Streams channel will be stored in in the user-states-database.
";

static PRINTLN_CHANNEL_STATUS_ABOUT: &str = "Print information about currently existing channels.
Each sensor is a subscriber in a dedicated Streams channel. The management-console
manages these channels and stores the channel state information in its
'user-states-management-console.sqlite3' database file. Use this CLI option to print
the relevant channel state information from the SQLite database to console.
Use CLI argument '--channel-starts-with' to select the Streams channel you want to investigate.
";

static CHANNEL_STARTS_WITH_ABOUT: &str = "Specify the Streams channel when processing a management-console
CLI command. As the Streams channels ID has 40 byte length and is
not easy to handle manually you only need to specify the beginning
of the channels ID so that it can be found in the user-states-database.
If there are more than one channels that can be found by the provided search string
the command will fail.

Example:

    >   ./management-console --channel-starts-with=6f5aa6cb --println-channel-status
";

static INIT_MULTIPLE_SENSORS_ABOUT: &str = "Initialize the streams channel of multiple sensors in parallel.
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

    >   ./management-console --init-multiple-sensors \\
                             --iota-bridge-url=\"http://192.168.47.11:50000\"
";

static INIT_SENSOR_ABOUT: &str = "Initialize the streams channel of a remote sensor.
The whole channel initialization is done automatically following the process described
below. Management-console and remote sensor are communicating via the IOTA-Bridge.
If your Sensor communicates with your IOTA-Bridge via an external domain or via an
external port of your local system, you will need to use the '--iota-bridge' option
to connect the Management-Console to the correct IOTA-Bridge.

Example:

  > ./management-console --init-sensor --iota-bridge-url=\"http://192.168.47.11:50000\"

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
";

static RUN_EXPLORER_API_SERVER_ABOUT: &str = "Run an http rest api server to explore sensor messages stored on the tangle
Default value for LISTENER_ADDRESS is 127.0.0.1:8080.

After the server has been started you may want to:
* fetch an overview about available paths from http://127.0.0.1:8080
* explore the swagger-ui open-api documentation via http://127.0.0.1:8080/swagger-ui

Example:

    >   ./management-console --run-explorer-api-server

Specify the listener address and port for the server like this:
Example:

    >   ./management-console --run-explorer-api-server 192.168.47.11:7777
";


static IOTA_BRIDGE_URL_ABOUT_FMT_STR: &str = "The url of the iota-bridge to connect to.
The default value will work together with the private tangle for development purposes
and a local running iota-bridge using the default settings.
See folder 'susee-node' for more details.

If your local iota-bridge listens to an external ip address, you need to specify this
address using the --iota-bridge-url argument.

If you are using an IOTA-Bridge provided by an external host, you need to specify the
domain or address using the --iota-bridge-url argument. For example use
\"http://iotabridge.peeros.de:50000\" for the SUSEE-Node provided by peerOS.

Default value is {}

Example: --iota-bridge-url=\"http://192.168.47.11:50000\"
";

static MANAGEMENTCONSOLE_APPLICATION_ABOUT: &str = "Management console for streams channels used in the SUSEE project.
Can be used to create new Streams channels and to add Sensors (a.k.a. Streams subscribers)
to those channels. Management of multiple channels is possible. The user states of the
Streams channels are stored in a local SQLite3 database file.
";

static DEV_EUI_ABOUT: &str = "The DevEUI of the sensor to act on.
DevEUI means 'device extended unique identifier' and is a term
from LoRaWAN communication. Any random value (number or string)
uniquely identifying the sensor can be used as long as the sensor
uses the same value.
";

pub type ManagementConsoleCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches<'a>() -> ArgMatchesAndOptions {
    let iota_bridge_url_about = String::from(IOTA_BRIDGE_URL_ABOUT_FMT_STR).replace("{}", STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);
    let arg_matches = ManagementConsoleCli::get_app(
            "Management Console",
            MANAGEMENTCONSOLE_APPLICATION_ABOUT,
            None,
        )
        .arg(Arg::new(ARG_KEYS.subscription_link)
            .long(ARG_KEYS.subscription_link)
            .short('l')
            .value_name("SUBSCRIPTION_LINK")
            .help(SUBSCRIPTION_LINK_ABOUT)
            .requires(ARG_KEYS.subscription_pub_key)
            .requires(ARG_KEYS.dev_eui)
            .conflicts_with_all(&[ARG_KEYS.init_sensor, ARG_KEYS.init_multiple_sensors, ARG_KEYS.create_channel, ARG_KEYS.run_explorer_api_server])
        )
        .arg(Arg::new(ARG_KEYS.subscription_pub_key)
            .long(ARG_KEYS.subscription_pub_key)
            .short('k')
            .value_name("SUBSCRIPTION_PUB_KEY")
            .help(SUBSCRIPTION_PUB_KEY_ABOUT)
            .requires(ARG_KEYS.subscription_link)
            .requires(ARG_KEYS.dev_eui)
            .conflicts_with_all(&[ARG_KEYS.init_sensor, ARG_KEYS.init_multiple_sensors, ARG_KEYS.create_channel, ARG_KEYS.run_explorer_api_server])
        )
        .arg(Arg::new(ARG_KEYS.create_channel)
            .long(ARG_KEYS.create_channel)
            .short('c')
            .help(CREATE_CHANNEL_ABOUT)
            .takes_value(false)
            .requires(ARG_KEYS.dev_eui)
            .conflicts_with_all(&[ARG_KEYS.init_sensor, ARG_KEYS.init_multiple_sensors, ARG_KEYS.subscription_pub_key, ARG_KEYS.subscription_link, ARG_KEYS.run_explorer_api_server])
        )
        .arg(Arg::new(ARG_KEYS.println_channel_status)
            .long(ARG_KEYS.println_channel_status)
            .short('p')
            .value_name("PRINTLN_CHANNEL_STATUS")
            .long_help(PRINTLN_CHANNEL_STATUS_ABOUT)
            .takes_value(false)
            .requires(ARG_KEYS.channel_starts_with)
        )
        .arg(Arg::new(ARG_KEYS.channel_starts_with)
            .long(ARG_KEYS.channel_starts_with)
            .short('s')
            .value_name("CHANNEL_STARTS_WITH")
            .help(CHANNEL_STARTS_WITH_ABOUT)
        )
        .arg(Arg::new(ARG_KEYS.init_sensor)
            .long(ARG_KEYS.init_sensor)
            .short('i')
            .help(INIT_SENSOR_ABOUT)
            .conflicts_with_all(&[ARG_KEYS.init_multiple_sensors, ARG_KEYS.create_channel, ARG_KEYS.subscription_pub_key, ARG_KEYS.subscription_link, ARG_KEYS.run_explorer_api_server])
            .takes_value(false)
        )
        .arg(Arg::new(ARG_KEYS.init_multiple_sensors)
            .long(ARG_KEYS.init_multiple_sensors)
            .short('m')
            .help(INIT_MULTIPLE_SENSORS_ABOUT)
            .conflicts_with_all(&[ARG_KEYS.init_sensor, ARG_KEYS.create_channel, ARG_KEYS.subscription_pub_key, ARG_KEYS.subscription_link, ARG_KEYS.run_explorer_api_server])
            .takes_value(false)
        )
        .arg(Arg::new(ARG_KEYS.run_explorer_api_server)
            .long(ARG_KEYS.run_explorer_api_server)
            .short('r')
            .help(RUN_EXPLORER_API_SERVER_ABOUT)
            .value_name("LISTENER_ADDRESS")
            .default_missing_value("127.0.0.1:8080")
            .conflicts_with_all(&[ARG_KEYS.init_sensor, ARG_KEYS.init_multiple_sensors, ARG_KEYS.create_channel, ARG_KEYS.subscription_pub_key, ARG_KEYS.subscription_link])
        )
        .arg(Arg::new(ARG_KEYS.iota_bridge_url)
            .long(ARG_KEYS.iota_bridge_url)
            .short('b')
            .value_name("IOTA_BRIDGE_URL")
            .help(iota_bridge_url_about.as_str())
        )
        .arg(Arg::new(ARG_KEYS.dev_eui)
            .long(ARG_KEYS.dev_eui)
            .short('e')
            .value_name("DEV_EUI")
            .help(DEV_EUI_ABOUT)
        )
        .get_matches();

    ArgMatchesAndOptions::new(arg_matches)
}