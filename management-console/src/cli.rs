use clap::{ArgMatches, Arg};

use susee_tools::{BaseArgKeys, BASE_ARG_KEYS, Cli};

pub struct ArgKeys {
    pub base: &'static BaseArgKeys,
    pub subscription_link: &'static str,
    pub subscription_pub_key: &'static str,
    pub create_channel: &'static str,
}

pub static ARG_KEYS: ArgKeys = ArgKeys {
    base: &BASE_ARG_KEYS,
    subscription_link: "subscription-link",
    subscription_pub_key: "subscription-pub-key",
    create_channel: "create-channel",
};

static SUBSCRIPTION_LINK_ABOUT: &str = "Subscription message link for the sensor subscriber.
Will be logged to console by the sensor app.
";

static SUBSCRIPTION_PUB_KEY_ABOUT: &str = "Public key of the sensor subscriber.
Will be logged to console by the sensor app.
";

static CREATE_CHANNEL_ABOUT: &str = "Use this option to create (announce) the channel.
The announcement link will be logged to the console.
";

pub type ManagementConsoleCli<'a> = Cli<'a, ArgKeys>;

pub fn get_arg_matches() -> ArgMatches {
    ManagementConsoleCli::get_app(
        "Management Console",
        "Management console for streams channels used in the SUSEE project"
    )
        .arg(Arg::new(ARG_KEYS.subscription_link)
            .short('l')
            .value_name("SUBSCRIPTION_LINK")
            .about(SUBSCRIPTION_LINK_ABOUT)
            //.required_unless_present(ARG_KEYS.create_channel)
        )
        .arg(Arg::new(ARG_KEYS.subscription_pub_key)
            .short('k')
            .value_name("SUBSCRIPTION_PUB_KEY")
            .about(SUBSCRIPTION_PUB_KEY_ABOUT)
            //.required_unless_present(ARG_KEYS.create_channel)
        )
        .arg(Arg::new(ARG_KEYS.create_channel)
            .short('c')
            .about(CREATE_CHANNEL_ABOUT)
           // .required_unless_present(ARG_KEYS.subscription_link)
            .takes_value(false)
        )
    .get_matches()
}