use clap::{ArgMatches};

use susee_tools::{BaseArgKeys, BASE_ARG_KEYS, Cli};
use susee_tools::cli_base::CliOptions;

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
};

pub type TangleProxyCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatches {
    TangleProxyCli::get_app(
            "Tangle Proxy",
            "Test tool to evaluate the behavior of the sensor counterpart proxy in the SUSEE \
            project which runs in the application server.",
            Some(CliOptions {
                use_wallet: false
            })
    )
    // TODO: New CLI option for adress: let address = "127.0.0.1:8080".parse().unwrap();
    .get_matches()
}