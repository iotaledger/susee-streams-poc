use clap::{ArgMatches, App, Arg};

static NODE_ABOUT: &str = "The url of the iota node to connect to.
Use 'https://chrysalis-nodes.iota.org' for the mainnet.

As there are several testnets have a look at
    https://wiki.iota.org/learn/networks/testnets
for alternative testnet urls.

Example:
    The iota chrysalis devnet: https://api.lb-0.h.chrysalis-devnet.iota.cafe
";

static FILES_TO_SEND_ABOUT: &str = "List of message files that will be encryped and send using the streams channel.
";

pub struct ArgKeys {
    pub node: &'static str,
    pub files_to_send: &'static str,
}

static ARG_KEYS: ArgKeys = ArgKeys {
    node: "node",
    files_to_send: "files-to-send",
};

pub fn get_arg_matches() -> ArgMatches {
    App::new("Streams Author Tool")
        .version("0.1.2")
        .author("Christof Gerritsma <christof.gerritsma@iota.org>")
        .about("Test tool to evaluate iota streams functionality for the SUSEE project")
        .arg(Arg::new(ARG_KEYS.node)
            .short('n')
            .value_name("NODE")
            .about(NODE_ABOUT)
            .default_value("https://chrysalis-nodes.iota.org")
        ).arg(Arg::new(ARG_KEYS.files_to_send)
            .short('f')
            .value_name("FILES_TO_SEND")
            .about(FILES_TO_SEND_ABOUT)
            .multiple_occurrences(true)
            .default_value("test/payloads/meter_reading-1-compact.json")
            .min_values(0)
    ).get_matches()
}

pub struct Cli<'a> {
    pub matches: &'a ArgMatches,
    pub arg_keys: &'static ArgKeys,
    pub node: &'a str,
}

impl<'a> Cli<'a> {
    pub fn new(arg_matches: &'a ArgMatches) -> Self {
        Self {
            matches: arg_matches,
            arg_keys: &ARG_KEYS,
            node: arg_matches.value_of("node").unwrap(),
        }
    }
}
