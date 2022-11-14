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
If needed you can use this option multiple times to specify several message files.";

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

static IOTA_BRIDGE_URL_ABOUT_FMT_STR: &str = "The url of the iota-bridge to connect to.
See --act-as-remote-control for further information.
Default value is {}

Example: --iota-bridge-url=\"http://192.168.47.11:50000\"";

static MOCK_REMOTE_SENSOR_ABOUT: &str = "Imitate (or mock) a remote sensor resp. an ESP32-Sensor
ESP32-Sensor here means the 'sensor/main-rust-esp-rs' (not the test app of
the streams-poc-lib).
This command is used to test x86/PC applications of the SUSEE streams POC in case
there are not enough ESP32 devices available. The sensor application will
periodically fetch and process commands from the iota-bridge.

If the iota-bridge runs on the same machine as this application, they can
communicate over the loopback IP address (localhost). In case the sensor
iota-bridge listens to the ip address of the network interface (the ip
address of the device that runs the iota-bridge) e.g. because some ESP32
sensors are also used you need to use CLI argument '--iota-bridge-url'
to specify this ip address.
";

static PRINTLN_SUBSCRIBER_STATUS_ABOUT: &str = "Print information about the current client status of the sensor.
In streams the sensor is a subscriber so that this client status is called subscriber status.
";

static CLEAR_CLIENT_STATE_ABOUT: &str = "Deletes the current client status of the sensor so that
all subscriptions get lost and the sensor can be used to subscribe to a new Streams channel.
TODO: In future versions the seed will also be replaced by a new generated seed.
TODO: -----------------------------
      --------  WARNING  ---------- Currently there is no confirmation cli dialog
      -----------------------------       use this option carefully!
";

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub files_to_send: &'static str,
    pub subscribe_announcement_link: &'static str,
    pub register_keyload_msg: &'static str,
    pub act_as_remote_control: &'static str,
    pub println_subscriber_status: &'static str,
    pub clear_client_state: &'static str,
    pub iota_bridge_url: &'static str,
    pub mock_remote_sensor: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    files_to_send: "file-to-send",
    subscribe_announcement_link: "subscribe-announcement-link",
    register_keyload_msg: "register-keyload-msg",
    act_as_remote_control: "act-as-remote-control",
    iota_bridge_url: "iota-bridge-url",
    clear_client_state: "clear-client-state",
    println_subscriber_status: "println-subscriber-status",
    mock_remote_sensor: "mock-remote-sensor",
};

pub type SensorCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatchesAndOptions {
    let iota_bridge_url_about = String::from(IOTA_BRIDGE_URL_ABOUT_FMT_STR).replace("{}", STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);

    let cli_opt = CliOptions {
        use_node: false,
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
            .arg(Arg::new(ARG_KEYS.iota_bridge_url)
                .long(ARG_KEYS.iota_bridge_url)
                .short('b')
                .value_name("IOTA_BRIDGE_URL")
                .help(iota_bridge_url_about.as_str())
            )
            .arg(Arg::new(ARG_KEYS.mock_remote_sensor)
                .long(ARG_KEYS.mock_remote_sensor)
                .short('m')
                .value_name("MOCK_REMOTE_SENSOR")
                .long_help(MOCK_REMOTE_SENSOR_ABOUT)
                .takes_value(false)
            )
            .arg(Arg::new(ARG_KEYS.println_subscriber_status)
                .long(ARG_KEYS.println_subscriber_status)
                .short('p')
                .value_name("PRINTLN_SUBSCRIBER_STATUS")
                .long_help(PRINTLN_SUBSCRIBER_STATUS_ABOUT)
                .takes_value(false)
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