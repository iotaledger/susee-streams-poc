use clap::{
    ArgMatches,
    Command,
    Arg
};

pub static NODE_ABOUT: &str = "The IP or domain name of the SUSEE Node to connect to.
Set this value to the domain name or static ip address of the SUSEE Node
which provides the IOTA Node, inx-collector and inx-poi web services.
See folder 'susee-node' for more details.

The IOTA Node and inx-collector API will be accessed using their
standard ports (14265 and 9030) automatically.

The default settings will connect to the private tangle that can be run
for development purposes (see folder 'susee-node' for more details).

Examples:
    --node=\"195.90.200.153\"
    -n=\"example.com\"
";

static WALLET_FILE_ABOUT_FMT_STR: &str = "Specifies the wallet file to use.
Set this to path and name of the wallet file.
If this option is not used:
* A file 'wallet-{}.txt' is used if existing
* If 'wallet-{}.txt' does not exist:
  A new seed is created and written into a new file
  'wallet-{}.txt'.
";

pub static DATA_DIR_ABOUT: &str = "The folder where all data files are stored.
This also applies for the location of the default wallet file (--wallet-file
argument is not used).
Examples:
    --data-dir=\"my_data/timestamp\"
    --data-dir=\"/home/admin/a_folder\"
";

static mut WALLET_FILE_ABOUT: String = String::new();
static mut DATA_DIR_DEFAULT_VALUE: String = String::new();

pub struct BaseArgKeys {
    pub node: &'static str,
    pub wallet_file: &'static str,
    pub data_dir: &'static str,
}

pub static BASE_ARG_KEYS: BaseArgKeys = BaseArgKeys {
    node: "node",
    wallet_file: "wallet-file",
    data_dir: "data-dir",
};

pub struct ProjectConstants {
    pub version: &'static str,
    pub author: &'static str,
    pub default_node: &'static str,
    pub default_data_dir_base_folder: &'static str,
}

pub static PROJECT_CONSTANTS: ProjectConstants = ProjectConstants {
    version: "0.1.2",
    author: "Christof Gerritsma <christof.gerritsma@iota.org>",
    default_node: "127.0.0.1",
    default_data_dir_base_folder: "data",
};

#[derive(Clone)]
pub struct CliOptions {
    pub use_node: bool,
    pub use_wallet: bool,
    pub use_data_dir: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            use_node: true,
            use_wallet: true,
            use_data_dir: true,
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
    pub node: &'a str,
    pub data_dir: String,
}

impl<'a, ArgKeysT> Cli<'a, ArgKeysT> {
    pub fn new(arg_match_and_opt: &'a ArgMatchesAndOptions, arg_keys: &'a ArgKeysT) -> Self {
        Self {
            options: arg_match_and_opt.options.clone(),
            matches: &arg_match_and_opt.matches,
            arg_keys,
            node: if arg_match_and_opt.options.use_node {
                    arg_match_and_opt.matches.value_of(BASE_ARG_KEYS.node).unwrap()
                } else {
                    "NONE"
                },
            data_dir: Self::get_data_dir_value(arg_match_and_opt),
        }
    }

    fn get_data_dir_value(arg_match_and_opt: &'a ArgMatchesAndOptions) -> String {
        let mut ret_val = ".".to_string();
        if arg_match_and_opt.options.use_data_dir {
            ret_val = arg_match_and_opt.matches.value_of(BASE_ARG_KEYS.data_dir).unwrap().to_string();
            if !ret_val.starts_with(".") && !ret_val.starts_with("/") {
                ret_val = format!("./{ret_val}");
            }
        }
        ret_val
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

        if options.use_data_dir {
            unsafe {
                DATA_DIR_DEFAULT_VALUE = format!("./{}/{}", PROJECT_CONSTANTS.default_data_dir_base_folder, app_name_lowercase);
                ret_val = ret_val.arg(Arg::new(BASE_ARG_KEYS.data_dir)
                    .long(BASE_ARG_KEYS.data_dir)
                    .short('d')
                    .value_name("DATA_DIR")
                    .help(DATA_DIR_ABOUT)
                    .default_value(DATA_DIR_DEFAULT_VALUE.as_str())
                );
            }
        }

        ret_val
    }
}