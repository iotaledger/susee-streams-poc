use clap::{
    ArgMatches,
    Arg
};

use susee_tools::{
    BaseArgKeys,
    BASE_ARG_KEYS,
    Cli
};

static FILE_TO_SEND_ABOUT: &str = "A message file that will be encrypted and send using the streams channel.
If needed you can use this option multiple times to specify several message files.";

static SUBSCRIBE_ANNOUNCEMENT_LINK_ABOUT: &str = "Subscribe to the channel via the specified announcement link.
";

static REGISTER_KEYLOAD_MSG_ABOUT: &str = "Register the specified keyload message so that it can be used
as root of the branch used to send messages later on.";

// TODO: Remove the node option because it is not used
// * Make it optional in CLI base
// * Remove it from README file

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub files_to_send: &'static str,
    pub subscribe_announcement_link: &'static str,
    pub register_keyload_msg: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    files_to_send: "file-to-send",
    subscribe_announcement_link: "subscribe-announcement-link",
    register_keyload_msg: "register-keyload-msg",
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
        .get_matches()
}