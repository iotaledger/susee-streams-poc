use clap::{
    ArgMatches,
    Command,
    Arg
};

pub static NODE_ABOUT: &str = "The url of the iota node to connect to.
Use 'https://chrysalis-nodes.iota.org' for the mainnet.

As there are several testnets have a look at
    https://wiki.iota.org/learn/networks/testnets
for alternative testnet urls.

Example:
    The iota chrysalis devnet:
    https://api.lb-0.h.chrysalis-devnet.iota.cafe
";

static WALLET_FILE_ABOUT_FMT_STR: &str = "Specifies the wallet file to use.
Set this to path and name of the wallet file.
If this option is not used:
* A file 'wallet-{}.txt' is used if existing
* If 'wallet-{}.txt' does not exist:
  A new seed is created and written into a new file
  'wallet-{}.txt'.
";

static mut WALLET_FILE_ABOUT: String = String::new();

pub struct BaseArgKeys {
    pub node: &'static str,
    pub wallet_file: &'static str,
}

pub static BASE_ARG_KEYS: BaseArgKeys = BaseArgKeys {
    node: "node",
    wallet_file: "wallet-file",
};

pub struct ProjectConstants {
    pub version: &'static str,
    pub author: &'static str,
    pub default_node: &'static str,
}

pub static PROJECT_CONSTANTS: ProjectConstants = ProjectConstants {
    version: "0.1.2",
    author: "Christof Gerritsma <christof.gerritsma@iota.org>",
    default_node: "https://chrysalis-nodes.iota.org",
};

#[derive(Clone)]
pub struct CliOptions {
    pub use_node: bool,
    pub use_wallet: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            use_node: true,
            use_wallet: true,
        }
    }
}


pub struct ArgMatchesAndOptions {
    pub options: CliOptions,
    pub matches: ArgMatches,
}

impl ArgMatchesAndOptions {
    pub fn new(matches: ArgMatches) -> Self {
        Self {
            options: CliOptions::default(),
            matches,
        }
    }
}


pub struct Cli<'a, ArgKeysT> {
    pub options: CliOptions,
    pub matches: &'a ArgMatches,
    pub arg_keys: &'a ArgKeysT,
    pub node: &'a str
}

impl<'a, ArgKeysT> Cli<'a, ArgKeysT> {
    pub fn new(arg_match_and_opt: &'a ArgMatchesAndOptions, arg_keys: &'a ArgKeysT) -> Self {
        Self {
            options: arg_match_and_opt.options.clone(),
            matches: &arg_match_and_opt.matches,
            arg_keys,
            node: if arg_match_and_opt.options.use_node {
                    arg_match_and_opt.matches.value_of("node").unwrap()
                } else {
                    "NONE"
                },
        }
    }

    pub fn get_app<'help>(name: &str, about: &'help str, options: Option<CliOptions> ) -> Command<'help> {
        let app_name_lowercase = name.to_lowercase().replace(" ", "-");
        let options = options.unwrap_or_default();
        let mut ret_val = Command::new(name)
            .version(PROJECT_CONSTANTS.version)
            .author(PROJECT_CONSTANTS.author)
            .about(about);

        if options.use_node {
            ret_val = ret_val.arg(Arg::new(BASE_ARG_KEYS.node)
                .long(BASE_ARG_KEYS.node)
                .short('n')
                .value_name("NODE_URL")
                .help(NODE_ABOUT)
                .default_value(PROJECT_CONSTANTS.default_node)
            );
        }

        if options.use_wallet {
            unsafe {
                WALLET_FILE_ABOUT = String::from(WALLET_FILE_ABOUT_FMT_STR).replace("{}", app_name_lowercase.as_str());
                ret_val = ret_val.arg(Arg::new(BASE_ARG_KEYS.wallet_file)
                    .long(BASE_ARG_KEYS.wallet_file)
                    .short('w')
                    .value_name("WALLET_FILE_PATH_AND_NAME")
                    .help(WALLET_FILE_ABOUT.as_str())
                );
            }
        }

        ret_val
    }
}