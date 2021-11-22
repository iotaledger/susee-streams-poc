use clap::{ArgMatches, App, Arg};

use susee_tools::{NODE_ABOUT, PROJECT_CONSTANTS, BaseArgKeys, BASE_ARG_KEYS, Cli};

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
};

pub fn get_arg_matches() -> ArgMatches {
    App::new("Tangle Proxy")
        .version(PROJECT_CONSTANTS.version)
        .author(PROJECT_CONSTANTS.author)
        .about("Test tool to evaluate the behavior of the sensor counterpart proxy in the SUSEE\
        project which runs in the application server ")
        .arg(Arg::new(ARG_KEYS.base.node)
            .short('n')
            .value_name("NODE")
            .about(NODE_ABOUT)
            .default_value(PROJECT_CONSTANTS.default_node)
        )
    .get_matches()
}

pub type TangleProxyCli<'a> = Cli<'a, ArgKeys>;