use clap::{ArgMatches};
use anyhow::Result;

use streams_tools::{
    PlainTextWallet
};

pub fn get_wallet(arg_matches: &ArgMatches, serialization_password: &str, wallet_file_arg_name: &str, default_wallet_file_name: &str) -> Result<PlainTextWallet>{
    let wallet_file_name: Option<&str>;
    if arg_matches.is_present(wallet_file_arg_name) {
        wallet_file_name = Some(arg_matches.value_of(wallet_file_arg_name).unwrap());
    } else {
        wallet_file_name = Some(default_wallet_file_name);
    }

    Ok(PlainTextWallet::new(serialization_password, wallet_file_name))
}