use std::{
    path::Path,
    fs
};

use clap::ArgMatches;
use anyhow::Result;

pub fn get_wallet_filename(arg_matches: &ArgMatches, wallet_file_arg_name: &str, data_dir: &String, default_wallet_file_name: &str) -> Result<String>{
    if arg_matches.is_present(wallet_file_arg_name) {
        Ok(arg_matches.value_of(wallet_file_arg_name).unwrap().to_string())
    } else {
        Ok(format!("{}/{}", data_dir, default_wallet_file_name))
    }
}

pub fn assert_data_dir_existence(data_dir: &String) -> Result<()>{
    if !Path::new(data_dir).exists() {
        fs::create_dir_all(data_dir)?;
    }
    Ok(())
}

pub fn get_data_folder_file_path(data_dir: &String, filename: &'static str) -> String {
    format!("{}/{}", data_dir, filename)
}