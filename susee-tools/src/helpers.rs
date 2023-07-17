use clap::ArgMatches;
use anyhow::Result;

pub fn get_wallet_filename(arg_matches: &ArgMatches, wallet_file_arg_name: &str, default_wallet_file_name: &str) -> Result<String>{
    if arg_matches.is_present(wallet_file_arg_name) {
        Ok(arg_matches.value_of(wallet_file_arg_name).unwrap().to_string())
    } else {
        Ok(String::from(default_wallet_file_name))
    }
}