use clap::{
    Arg
};

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli,
    cli_base::ArgMatchesAndOptions,
};

use streams_tools::STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL;
use susee_tools::cli_base::CliOptions;

static FILE_TO_SEND_ABOUT: &str = "A message file that will be encrypted and send using the streams channel.
The message will be resend every 5 Seconds in an endless loop.
Use CTRL-C to stop processing.";

static RANDOM_MSG_OF_SIZE_ABOUT: &str = "A random message of the specified length in bytes will be created,
encrypted and send using the streams channel.
A new random message will be resend every 5 Seconds in an endless loop.
Use CTRL-C to stop processing.";

static SUBSCRIBE_ANNOUNCEMENT_LINK_ABOUT: &str = "Subscribe to the channel via the specified announcement link.
";

static REGISTER_KEYLOAD_MSG_ABOUT: &str = "Register the specified keyload message so that it can be used
as root of the branch used to send messages later on.";

static ACT_AS_REMOTE_CONTROL_ABOUT: &str = "Use this argument to remotely control a running sensor application on
an embedded device. For example this

  > ./sensor --subscribe-announcement-link \"c67551dade.....6daff2\"\\
             --act-as-remote-control --iota-bridge-url=\"http://192.168.47.11:50000\"

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
";

static ACT_AS_REMOTE_CONTROLLED_SENSOR_ABOUT: &str = "\
Imitate a remote sensor resp. an ESP32-Sensor awaiting remote control commands.
ESP32-Sensor here means the 'sensor/main-rust-esp-rs' application or the
test app of the streams-poc-lib in an initial Streams channel state.

This command is used to test the iota-bridge and the management-console application
in case there are not enough ESP32 devices available. The sensor application will
periodically fetch and process commands from the iota-bridge.

The only usecase for this CLI argument currently is to initialize a sensor
fully automatically using the --init-sensor argument of the management-console.
Therefore the sensor will do a DevEUI-Handshake with the management-console
via the iota-bridge at the beginning of the command fetch process.

To perform the DevEUI-Handshake the sensor will use the dev_eui 'ANY'
to fetch a DevEuiHandshake command from the iota-bridge.
It will not accept any other type of command and will fetch commands
until a DevEuiHandshake command has been received.
After a DevEuiHandshake command has been received, the sensor
will send its real dev_eui to the management-console with the
DevEuiHandshake confirmation via the iota-bridge. This is followed by
fetch command, process command and confirm sequences using the dev_eui
of the sensor.

If the iota-bridge runs on the same machine as this application, they can
communicate over the loopback IP address (localhost). In case the sensor
iota-bridge listens to the ip address of the network interface (the ip
address of the device that runs the iota-bridge) e.g. because some ESP32
sensors are also used, you need to use the CLI argument '--iota-bridge-url'
to specify this ip address.
";

static EXIT_AFTER_SUCCESSFUL_INITIALIZATION_ABOUT: &str = "\
If specified in combination with --act-as-remote-controlled-sensor the command poll loop
will be stopped after a KEYLOAD_REGISTRATION confirmation has been send to confirm
a successfully processed REGISTER_KEYLOAD_MESSAGE command.
This argument is useful when the sensor app runs in automation scripts to allow the
initialization of the Sensor and the Sensor app should exit after successful initialization.
";

static USE_LORAWAN_REST_API_ABOUT: &str = "\
If used the Sensor application will not call iota-bridge API functions directly
but will use its lorawan-rest API instead.
This way the Sensor application imitates the behavior of an ESP32-Sensor connected
via LoRaWAN and an 'Application Server Connector' that hands over binary packages
to the iota-bridge.";

static IOTA_BRIDGE_URL_ABOUT_FMT_STR: &str = "The url of the iota-bridge to connect to.
See --act-as-remote-control for further information.
Default value is {}

Example: --iota-bridge-url=\"http://192.168.47.11:50000\"";

static FAILOVER_IOTA_BRIDGE_URL_ABOUT: &str = "\
Specifies a secondary iota-bridge used for failover.
In case the primary iota-bridge (specified by the CLI argument '--iota-bridge-url')
returns an erroneous http response, the sensor will try to use a secondary
iota-bridge instance specified by this argument.
The implemented failover-handling is very simple: The secondary iota-bridge is only
called in case of errors and only once (per error).

If this argument is not provided, no failover is done.

The error response behavior of the iota-bridge is designed to allow failover-handling
run on an application-server-connector. Therefore this argument must be used together
with the --use-lorawan-rest-api argument. In theory, it would be possible to use
--failover-iota-bridge-url while accessing iota-bridge API functions directly,
but this would break the communication between the iota-bridge and the Sensor.

Example: --failover-iota-bridge-url=\"http://192.168.47.11:50000\"
";

static PRINTLN_SUBSCRIBER_STATUS_ABOUT: &str = "Print information about the current client status of the sensor.
In streams the sensor is a subscriber so that this client status is called subscriber status.
";

static DEV_EUI_ABOUT: &str = "Use the specified LoRaWAN DevEui instead of a random value.
In case the sensor wallet file (wallet-sensor.txt) of the sensor has been deleted,
the default behavior is to use a random value as new DevEui.
The generated DevEui then is stored in the sensor wallet file later on so that the
DevEui is persisted for later use.
Using this argument the DevEui can be pre defined to have a static DevEui for test
purposes. This argument is ignored in case the DevEui has already been stored in the
sensor wallet file.

Example: --dev-eui=12345678
";

static CLEAR_CLIENT_STATE_ABOUT: &str = "Deletes the current client status of the sensor so that
all subscriptions get lost and the sensor can be used to subscribe to a new Streams channel.
TODO: -----------------------------
      --------  WARNING  ---------- Currently there is no confirmation cli dialog
      -----------------------------       use this option carefully!
";

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub dev_eui: &'static str,
    pub files_to_send: &'static str,
    pub random_msg_of_size: &'static str,
    pub subscribe_announcement_link: &'static str,
    pub register_keyload_msg: &'static str,
    pub act_as_remote_control: &'static str,
    pub act_as_remote_controlled_sensor: &'static str,
    pub println_subscriber_status: &'static str,
    pub clear_client_state: &'static str,
    pub iota_bridge_url: &'static str,
    pub use_lorawan_rest_api: &'static str,
    pub failover_iota_bridge_url: &'static str,
    pub exit_after_successful_initialization: &'static str,
}



pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    dev_eui: "dev-eui",
    files_to_send: "file-to-send",
    random_msg_of_size: "random-msg-of-size",
    subscribe_announcement_link: "subscribe-announcement-link",
    register_keyload_msg: "register-keyload-msg",
    act_as_remote_control: "act-as-remote-control",
    act_as_remote_controlled_sensor: "act-as-remote-controlled-sensor",
    iota_bridge_url: "iota-bridge-url",
    failover_iota_bridge_url: "failover-iota-bridge-url",
    clear_client_state: "clear-client-state",
    println_subscriber_status: "println-subscriber-status",
    use_lorawan_rest_api: "use-lorawan-rest-api",
    exit_after_successful_initialization: "exit-after-successful-initialization",
};

pub type SensorCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatchesAndOptions {
    let iota_bridge_url_about = String::from(IOTA_BRIDGE_URL_ABOUT_FMT_STR).replace("{}", STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);

    let cli_opt = CliOptions {
        use_node: false,
        use_data_dir: false,
        use_wallet: true,
    };

    let arg_matches = SensorCli::get_app(
            "Sensor",
            "Test tool to evaluate sensor behavior in the SUSEE project",
            Some(cli_opt.clone()),
        )
            .arg(Arg::new(ARG_KEYS.subscribe_announcement_link)
                .long(ARG_KEYS.subscribe_announcement_link)
                .short('s')
                .value_name("SUBSCRIBE_ANNOUNCEMENT_LINK")
                .long_help(SUBSCRIBE_ANNOUNCEMENT_LINK_ABOUT)
                .conflicts_with(ARG_KEYS.register_keyload_msg)
                .conflicts_with(ARG_KEYS.files_to_send)
            )
            .arg(Arg::new(ARG_KEYS.register_keyload_msg)
                .long(ARG_KEYS.register_keyload_msg)
                .short('r')
                .value_name("KEYLOAD_MSG_LINK")
                .long_help(REGISTER_KEYLOAD_MSG_ABOUT)
                .conflicts_with(ARG_KEYS.subscribe_announcement_link)
                .conflicts_with(ARG_KEYS.files_to_send)
            )
            .arg(Arg::new(ARG_KEYS.files_to_send)
                .long(ARG_KEYS.files_to_send)
                .short('f')
                .value_name("FILE_TO_SEND")
                .long_help(FILE_TO_SEND_ABOUT)
                .multiple_occurrences(true)
                .min_values(0)
                .conflicts_with(ARG_KEYS.random_msg_of_size)
                .conflicts_with(ARG_KEYS.subscribe_announcement_link)
                .conflicts_with(ARG_KEYS.register_keyload_msg)
            )
            .arg(Arg::new(ARG_KEYS.random_msg_of_size)
                .long(ARG_KEYS.random_msg_of_size)
                .short('g')
                .value_name("MSG_SIZE")
                .long_help(RANDOM_MSG_OF_SIZE_ABOUT)
                .conflicts_with(ARG_KEYS.files_to_send)
                .conflicts_with(ARG_KEYS.subscribe_announcement_link)
                .conflicts_with(ARG_KEYS.register_keyload_msg)
            )
            .arg(Arg::new(ARG_KEYS.act_as_remote_control)
                .long(ARG_KEYS.act_as_remote_control)
                .short('c')
                .value_name("ACT_AS_REMOTE_CONTROL")
                .long_help(ACT_AS_REMOTE_CONTROL_ABOUT)
                .takes_value(false)
            )
            .arg(Arg::new(ARG_KEYS.act_as_remote_controlled_sensor)
                .long(ARG_KEYS.act_as_remote_controlled_sensor)
                .short('m')
                .value_name("ACT_AS_REMOTE_CONTROLLED_SENSOR")
                .long_help(ACT_AS_REMOTE_CONTROLLED_SENSOR_ABOUT)
                .takes_value(false)
            )
            .arg(Arg::new(ARG_KEYS.iota_bridge_url)
                .long(ARG_KEYS.iota_bridge_url)
                .short('b')
                .value_name("IOTA_BRIDGE_URL")
                .help(iota_bridge_url_about.as_str())
            )
            .arg(Arg::new(ARG_KEYS.failover_iota_bridge_url)
                .long(ARG_KEYS.failover_iota_bridge_url)
                .short('o')
                .value_name("FAILOVER_IOTA_BRIDGE_URL")
                .requires(ARG_KEYS.use_lorawan_rest_api)
                .help(FAILOVER_IOTA_BRIDGE_URL_ABOUT)
            )
            .arg(Arg::new(ARG_KEYS.dev_eui)
                .long(ARG_KEYS.dev_eui)
                .short('d')
                .value_name("DEV_EUI")
                .help(DEV_EUI_ABOUT)
            )
            .arg(Arg::new(ARG_KEYS.println_subscriber_status)
                .long(ARG_KEYS.println_subscriber_status)
                .short('p')
                .value_name("PRINTLN_SUBSCRIBER_STATUS")
                .long_help(PRINTLN_SUBSCRIBER_STATUS_ABOUT)
                .takes_value(false)
            )
            .arg(Arg::new(ARG_KEYS.use_lorawan_rest_api)
                .long(ARG_KEYS.use_lorawan_rest_api)
                .short('l')
                .value_name("USE_LORAWAN_REST_API")
                .long_help(USE_LORAWAN_REST_API_ABOUT)
                .takes_value(false)
            )
            .arg(Arg::new(ARG_KEYS.exit_after_successful_initialization)
                .long(ARG_KEYS.exit_after_successful_initialization)
                .short('e')
                .value_name("EXIT_AFTER_SUCCESSFUL_INITIALIZATION")
                .long_help(EXIT_AFTER_SUCCESSFUL_INITIALIZATION_ABOUT)
                .takes_value(false)
                .requires(ARG_KEYS.act_as_remote_controlled_sensor)
            )
            .arg(Arg::new(ARG_KEYS.clear_client_state)
                .long(ARG_KEYS.clear_client_state)
                .value_name("CLEAR_CLIENT_STATE")
                .long_help(CLEAR_CLIENT_STATE_ABOUT)
                .takes_value(false)
            )
            .get_matches();

    ArgMatchesAndOptions {
        options: cli_opt,
        matches: arg_matches,
    }
}