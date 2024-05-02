use clap::{
    Arg
};

use streams_tools::iota_bridge::iota_bridge::ErrorHandlingStrategy;

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli,
    cli_base::{
        CliOptions,
        ArgMatchesAndOptions
    }
};

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub listener_ip_address_port: &'static str,
    pub error_handling: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    listener_ip_address_port: "listener-ip-address",
    error_handling: "error-handling",
};

static LISTENER_IP_ADDRESS_PORT_ABOUT: &str = "IP address and port to listen to.
Example: listener-ip-address=\"192.168.47.11:50000\"
";

static ERROR_HANDLING_ABOUT_FMT_STR: &str = "Defines how errors occurring during 'lorawan-rest/binary_request'
endpoint processing are handled.

{}

Internal errors of the iota-bridge are provided via http error status codes:

    | ------------------------------ | --------------------------- |
    | Error Type                     | HTTP Error Status           |
    | ------------------------------ | --------------------------- |
    | *SUSEE Node* health error      | 503 - Service Unavailable   |
    | Message send validation error  | 507 - Insufficient Storage  |
    | Other error                    | 500 - Internal Server Error |
    | ------------------------------ | --------------------------- |

For more details regarding the different error types please see the
iota-bridge Readme.md file.
";

pub type IotaBridgeCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatchesAndOptions {
    let cli_opt = CliOptions {
        use_node: true,
        use_data_dir: true,
        use_wallet: false,
    };
    let error_handling_about = String::from(ERROR_HANDLING_ABOUT_FMT_STR).replace("{}", ErrorHandlingStrategy::DESCRIPTION);
    let arg_matches = IotaBridgeCli::get_app(
            "IOTA Bridge",
            "Test tool to evaluate the behavior of the sensor counterpart proxy in the SUSEE \
            project which runs in the application server.",
            Some(cli_opt.clone()),
        )
        .arg(Arg::new(ARG_KEYS.listener_ip_address_port)
            .long(ARG_KEYS.listener_ip_address_port)
            .short('l')
            .value_name("LISTENER_IP_ADDRESS_PORT")
            .help(LISTENER_IP_ADDRESS_PORT_ABOUT)
        )
        .arg(Arg::new(ARG_KEYS.error_handling)
            .long(ARG_KEYS.error_handling)
            .short('e')
            .value_name("ERROR_HANDLING")
            .default_value(ErrorHandlingStrategy::DEFAULT)
            .help(error_handling_about.as_str())
        )
        .get_matches();

    ArgMatchesAndOptions {
        options: cli_opt,
        matches: arg_matches,
    }
}