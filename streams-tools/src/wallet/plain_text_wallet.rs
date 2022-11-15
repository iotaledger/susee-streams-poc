use std::path::Path;
use std::{
    fs::write as fs_write,
    fs::read as fs_read,
    hash::Hasher,
    num::Wrapping,
    collections::hash_map::DefaultHasher
};
use rand::Rng;
use anyhow::Result;
use std::string::FromUtf8Error;

use super::simple_wallet::SimpleWallet;

const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";
const ALPH9_LEN: usize = 27;

static DEFAULT_FILE_NAME: &str = "channel-seed.txt";

// #################################################################################################
//
//                          ----------------------------------------------
//                          DO NOT USE THIS WALLET FOR PRODUCTION PURPOSES
//                          ----------------------------------------------
//
//     Instead implement the SimpleWallet trait using a secure wallet library like stronghold.
//
// #################################################################################################

pub struct PlainTextWallet {
    pub file_name: String,
    pub seed: String,
    pub serialization_password: String,
    pub derived_seed: Option<String>,
    pub seed_derivation_phrase: Option<String>,
}

pub fn create_seed() -> String {
    let seed: &str = &(0..81)
        .map(|_| {
            ALPH9
                .chars()
                .nth(rand::thread_rng().gen_range(0, ALPH9_LEN))
                .unwrap()
        })
        .collect::<String>();
    String::from(seed)
}

fn create_seed_from_derivation_phrase(master_seed: &str, seed_derivation_phrase: &str) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write_str(master_seed);
    hasher.write_str(seed_derivation_phrase);
    let hash = hasher.finish();
    let w_hash = Wrapping(hash);

    let derived_seed: &str = &(1..82)
        .map(|pos| {
            let char_indx = (w_hash * Wrapping(pos)).0 % ALPH9_LEN as u64;
            ALPH9
                .chars()
                .nth(char_indx as usize)
                .unwrap()
        })
        .collect::<String>();

    println!("seed_derivation_phrase: {}", derived_seed);
    String::from(derived_seed)
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
    pub fn new(serialization_password: &str, file_path_name: Option<&str>, seed_derivation_phrase: Option<String>) -> Self{
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
        let mut derived_seed: Option<String> = None;
        if let Some(derivation_phrase) = seed_derivation_phrase.as_ref() {
            derived_seed = Some(create_seed_from_derivation_phrase(seed.as_str(), derivation_phrase.as_str()));
        }
        Self{
            file_name: String::from(file_name),
            seed,
            serialization_password: String::from(serialization_password),
            derived_seed,
            seed_derivation_phrase,
        }
    }
}

impl SimpleWallet for PlainTextWallet {
    const IS_USABLE_WALLET: bool = true;

    fn get_seed(&self) -> &str {
        if let Some(derived_seed) = self.derived_seed.as_ref() {
            derived_seed.as_str()
        } else {
            self.seed.as_str()
        }
    }

    fn get_serialization_password(&self) -> &str {
        self.serialization_password.as_str()
    }
}