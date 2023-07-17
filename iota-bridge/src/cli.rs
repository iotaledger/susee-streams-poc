use clap::{
    Arg
};

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli
};
use susee_tools::cli_base::{
    CliOptions,
    ArgMatchesAndOptions
};

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub listener_ip_address_port: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    listener_ip_address_port: "listener-ip-address",
};

static LISTENER_IP_ADDRESS_PORT_ABOUT: &str = "IP address and port to listen to.
Example: listener-ip-address=\"192.168.47.11:50000\"
";

pub type IotaBridgeCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatchesAndOptions {
    let cli_opt = CliOptions {
        use_node: true,
        use_wallet: false,
    };
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
        .get_matches();

    ArgMatchesAndOptions {
        options: cli_opt,
        matches: arg_matches,
    }
}