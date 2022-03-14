use std::path::Path;
use std::{
    fs::write as fs_write,
    fs::read as fs_read,
};
use rand::Rng;
use anyhow::Result;
use std::string::FromUtf8Error;

use super::simple_wallet::SimpleWallet;

const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";

static DEFAULT_FILE_NAME: &str = "channel-seed.txt";

pub struct PlainTextWallet {
    pub file_name: String,
    pub seed: String,
    pub serialization_password: String,
}

pub fn create_seed() -> String {
    let seed: &str = &(0..81)
        .map(|_| {
            ALPH9
                .chars()
                .nth(rand::thread_rng().gen_range(0, 27))
                .unwrap()
        })
        .collect::<String>();
    String::from(seed)
}

fn create_seed_file(file_name: &str) -> Result<String>{
    let seed = create_seed();
    fs_write(file_name, &seed).expect(format!("Could not create seed file '{}'", file_name).as_str());
    Ok(seed.clone())
}

fn read_seed_file(file_name: &str) -> std::result::Result<String, FromUtf8Error> {
    let buffer = fs_read(file_name).expect(format!("Could not open seed file '{}'", file_name).as_str());
    String::from_utf8(buffer)
}

impl PlainTextWallet {
    pub fn new(serialization_password: &str, file_path_name: Option<&str>) -> Self{
        let file_name: &str;
        match file_path_name {
            Some(name) => file_name = name,
            _ => file_name = DEFAULT_FILE_NAME,
        }
        let seed: String;
        if Path::new(file_name).exists(){
            seed = read_seed_file(file_name).unwrap_or(String::from(format!("Could not open seed file '{}'", file_name)));
        } else {
            seed = create_seed_file(file_name).unwrap_or(String::from(format!("Could not create seed file '{}'", file_name)));
        }
        Self{
            file_name: String::from(file_name),
            seed,
            serialization_password: String::from(serialization_password),
        }
    }
}

impl SimpleWallet for PlainTextWallet {
    const IS_USABLE_WALLET: bool = true;
    fn get_seed(&self) -> &str {
        self.seed.as_str()
    }
    fn get_serialization_password(&self) -> &str {
        self.serialization_password.as_str()
    }
}