use clap::{ArgMatches, Arg};

use susee_tools::{BaseArgKeys, BASE_ARG_KEYS, Cli};

static FILES_TO_SEND_ABOUT: &str = "List of message files that will be encrypted and send using the streams channel.
";

static SUBSCRIBE_ANNOUNCEMENT_LINK_ABOUT: &str = "Subscribe to the channel via the specified announcement link.
";

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub files_to_send: &'static str,
    pub subscribe_announcement_link: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    files_to_send: "files-to-send",
    subscribe_announcement_link: "announcement-link-subscribe",
};

pub type SensorCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatches {
    SensorCli::get_app(
        "Sensor",
        "Test tool to evaluate sensor behavior in the SUSEE project",
        None,
    )
    .arg(Arg::new(ARG_KEYS.subscribe_announcement_link)
        .long(ARG_KEYS.subscribe_announcement_link)
        .short('s')
        .value_name("ANNOUNCEMENT_LINK_SUBSCRIBE")
        .about(SUBSCRIBE_ANNOUNCEMENT_LINK_ABOUT)
    )
    .arg(Arg::new(ARG_KEYS.files_to_send)
        .long(ARG_KEYS.files_to_send)
        .short('f')
        .value_name("FILES_TO_SEND")
        .about(FILES_TO_SEND_ABOUT)
        .multiple_occurrences(true)
        .default_value("payloads/meter_reading-1-compact.json")
        .min_values(0)
    )
    .get_matches()
}