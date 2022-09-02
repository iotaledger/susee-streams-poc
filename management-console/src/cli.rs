use clap::{
    ArgMatches,
    Arg
};

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli
};
use streams_tools::STREAMS_TOOLS_CONST_HTTP_PROXY_URL;

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub subscription_link: &'static str,
    pub subscription_pub_key: &'static str,
    pub create_channel: &'static str,
    pub init_sensor: &'static str,
    pub iota_bridge_url: &'static str,
    pub println_channel_status: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    subscription_link: "subscription-link",
    subscription_pub_key: "subscription-pub-key",
    create_channel: "create-channel",
    init_sensor: "init-sensor",
    iota_bridge_url: "iota-bridge-url",
    println_channel_status: "println-channel-status",
};

static SUBSCRIPTION_LINK_ABOUT: &str = "Subscription message link for the sensor subscriber.
Will be logged to console by the sensor app.
";

static SUBSCRIPTION_PUB_KEY_ABOUT: &str = "Public key of the sensor subscriber.
Will be logged to console by the sensor app.
";

static CREATE_CHANNEL_ABOUT: &str = "Use this option to create (announce) the channel.
The announcement link will be logged to the console.
";

static PRINTLN_CHANNEL_STATUS_ABOUT: &str = "Print information about all currently existing channels.
Each sensor is a subscriber in a dedicated Streams channel. The management-console
manages these channels and stores the channel state information in its
'user-state-management-console.sqlite' database file. Use this CLI option to print
the relevant channel state information from the SQLite database to console.
";

static INIT_SENSOR_ABOUT: &str = "Initialize the streams channel of a remote sensor.
The whole channel initialization is done automatically following the process described
below. Management-console and remote sensor are communicating via the IOTA-Bridge.
Therefore you also need to use the '--iota-bridge' option to connect the management-
console to a running IOTA-Bridge.

Example:

  > ./management-console --init-sensor --iota-bridge-url=\"http://192.168.47.11:50000\"

Please make sure that the remote sensor and the management-console have a working
connection to the running iota-bridge.

Initialization Process
----------------------
The below mentioned Commands and Confirmations are used for process communication
with the remote sensor via the IOTA-Bridge and are defined in the binary_persist
module of the streams-tools library:

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

static IOTA_BRIDGE_URL_ABOUT_FMT_STR: &str = "The url of the iota-bridge to connect to.
See --init-sensor for further information.
Default value is {}

Example: iota-bridge-url=\"http://192.168.47.11:50000\"";

pub type ManagementConsoleCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatches {
    let iota_bridge_url_about = String::from(IOTA_BRIDGE_URL_ABOUT_FMT_STR).replace("{}", STREAMS_TOOLS_CONST_HTTP_PROXY_URL);
    ManagementConsoleCli::get_app(
        "Management Console",
        "Management console for streams channels used in the SUSEE project",
        None,
    )
    .arg(Arg::new(ARG_KEYS.subscription_link)
        .long(ARG_KEYS.subscription_link)
        .short('l')
        .value_name("SUBSCRIPTION_LINK")
        .help(SUBSCRIPTION_LINK_ABOUT)
        .requires(ARG_KEYS.subscription_pub_key)
    )
    .arg(Arg::new(ARG_KEYS.subscription_pub_key)
        .long(ARG_KEYS.subscription_pub_key)
        .short('k')
        .value_name("SUBSCRIPTION_PUB_KEY")
        .help(SUBSCRIPTION_PUB_KEY_ABOUT)
        .requires(ARG_KEYS.subscription_link)
    )
    .arg(Arg::new(ARG_KEYS.create_channel)
        .long(ARG_KEYS.create_channel)
        .short('c')
        .help(CREATE_CHANNEL_ABOUT)
        .takes_value(false)
    )
    .arg(Arg::new(ARG_KEYS.println_channel_status)
        .long(ARG_KEYS.println_channel_status)
        .short('p')
        .value_name("PRINTLN_CHANNEL_STATUS")
        .long_help(PRINTLN_CHANNEL_STATUS_ABOUT)
        .takes_value(false)
    )
    .arg(Arg::new(ARG_KEYS.init_sensor)
        .long(ARG_KEYS.init_sensor)
        .short('i')
        .help(INIT_SENSOR_ABOUT)
        .requires(ARG_KEYS.iota_bridge_url)
        .takes_value(false)
    )
    .arg(Arg::new(ARG_KEYS.iota_bridge_url)
        .long(ARG_KEYS.iota_bridge_url)
        .short('b')
        .value_name("IOTA_BRIDGE_URL")
        .help(iota_bridge_url_about.as_str())
        .requires(ARG_KEYS.init_sensor)
    )
    .get_matches()
}