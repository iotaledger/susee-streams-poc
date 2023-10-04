use clap::{
    Arg
};

use susee_tools::{
    cli_base::{
        CliOptions,
        ArgMatchesAndOptions,
    },
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli
};

use streams_tools::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS,
};

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub listener_ip_address_port: &'static str,
    pub iota_bridge_url: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    listener_ip_address_port: "listener-ip-address",
    iota_bridge_url: "iota-bridge-url",
};

static LISTENER_IP_ADDRESS_PORT_ABOUT: &str = "IP address and port to listen to.
Example: listener-ip-address=\"192.168.47.11:50001\"

DO NOT USE THE SAME PORT FOR THE IOTA-BRIDGE AND THIS APPLICATION
";

static IOTA_BRIDGE_URL_ABOUT_FMT_STR: &str = "The url of the iota-bridge to connect to.
Default value is {}
Example: iota-bridge-url=\"http://192.168.47.11:50000\"";

pub type LoraWanAppServerMockCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatchesAndOptions {
    let iota_bridge_url_about = String::from(IOTA_BRIDGE_URL_ABOUT_FMT_STR).replace("{}", STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);
    let cli_opt = CliOptions {
        use_node: false,
        use_data_dir: false,
        use_wallet: false,
    };
    let arg_matches = LoraWanAppServerMockCli::get_app(
            "AppServer Connector Mockup Tool",
            "
            This is a test tool to receive binary packages from an 'ESP32 Sensor' via a socket
            connection and post the package to the *IOTA-Bridge* via its `lorawan-rest`
            API functions.
            This application is only needed if you use the test application provided with the
            streams-poc-lib implemented in the file sensor/streams-poc-lib/main/main.c",
            Some(cli_opt.clone())
        )
        .arg(Arg::new(ARG_KEYS.listener_ip_address_port)
            .long(ARG_KEYS.listener_ip_address_port)
            .short('l')
            .value_name("LISTENER_IP_ADDRESS_PORT")
            .help(LISTENER_IP_ADDRESS_PORT_ABOUT)
            .default_value(STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS)
        )
        .arg(Arg::new(ARG_KEYS.iota_bridge_url)
            .long(ARG_KEYS.iota_bridge_url)
            .short('b')
            .value_name("IOTA_BRIDGE_URL")
            .help(iota_bridge_url_about.as_str())
            .default_value(STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL)
        )
        .get_matches();
    ArgMatchesAndOptions {
        options: cli_opt,
        matches: arg_matches,
    }
}