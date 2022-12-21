use std::{
    fs::write as fs_write,
    fs::read as fs_read,
    path::Path,
    hash::Hasher,
    num::Wrapping,
    collections::hash_map::DefaultHasher,
    ops::Range,
};

use crate::binary_persist::{
    BinaryPersist,
    USIZE_LEN,
    RangeIterator,
    serialize_string,
    deserialize_string
};

use super::simple_wallet::SimpleWallet;

use rand::Rng;

use anyhow::Result;

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

pub struct PtwPersist {
    pub seed: String,
    pub misc_other_data: String,
}

impl BinaryPersist for PtwPersist {
    fn needed_size(&self) -> usize {
        let mut ret_val = 2 * USIZE_LEN; // Length of 2 Strings: seed + misc_other_data
        ret_val += self.seed.len();
        ret_val += self.misc_other_data.len();
        ret_val
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinaryPersist for PtwPersist - to_bytes()] Need {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_string(&self.seed, buffer, &mut range)?;
        serialize_string(&self.misc_other_data, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(0);
        let seed= deserialize_string(buffer, & mut range)?;
        let misc_other_data= deserialize_string(buffer, & mut range)?;
        Ok(PtwPersist {seed, misc_other_data })
    }
}

pub struct PlainTextWallet {
    pub file_name: String,
    pub persist: PtwPersist,
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

fn create_persistence_file(file_name: &str) -> Result<PtwPersist>{
    let persist = PtwPersist{
        seed: create_seed(),
        misc_other_data: String::default(),
    };
    write_persistence_file(file_name, &persist)?;
    Ok(persist)
}

fn write_persistence_file(file_name: &str, persist: &PtwPersist) -> Result<()>{
    let mut buffer = vec![0_u8; persist.needed_size()];
    let _data_len = persist.to_bytes(&mut buffer).expect("Error on persisting PtwPersist to binary buffer");
    fs_write(file_name, buffer.as_slice()).expect(format!("Could not create persistence file '{}'", file_name).as_str());
    Ok(())
}

fn read_persistence_file(file_name: &str) -> Result<PtwPersist> {
    let buffer = fs_read(file_name).expect(format!("Could not open persistence file '{}'", file_name).as_str());
    PtwPersist::try_from_bytes(buffer.as_slice())
}

impl PlainTextWallet {
    pub fn new(serialization_password: &str, file_path_name: Option<&str>, seed_derivation_phrase: Option<String>) -> Self{
        let file_name: &str;
        match file_path_name {
            Some(name) => file_name = name,
            _ => file_name = DEFAULT_FILE_NAME,
        }
        let ptw_persist: PtwPersist;
        if Path::new(file_name).exists(){
            ptw_persist = read_persistence_file(file_name).expect(format!("Error while processing the persistence file '{}'", file_name).as_str());
        } else {
            ptw_persist = create_persistence_file(file_name).expect(format!("Error on creating the persistence file '{}'", file_name).as_str());
        }
        let mut derived_seed: Option<String> = None;
        if let Some(derivation_phrase) = seed_derivation_phrase.as_ref() {
            derived_seed = Some(create_seed_from_derivation_phrase(ptw_persist.seed.as_str(), derivation_phrase.as_str()));
        }
        Self{
            file_name: String::from(file_name),
            serialization_password: String::from(serialization_password),
            derived_seed,
            seed_derivation_phrase,
            persist: ptw_persist,
        }
    }

    pub fn write_wallet_file(&self) {
        write_persistence_file(self.file_name.as_str(), &self.persist)
            .expect(format!("Error on writing the persistence file '{}'", self.file_name).as_str());
    }
}

impl SimpleWallet for PlainTextWallet {
    const IS_USABLE_WALLET: bool = true;

    fn get_seed(&self) -> &str {
        if let Some(derived_seed) = self.derived_seed.as_ref() {
            derived_seed.as_str()
        } else {
            self.persist.seed.as_str()
        }
    }

    fn get_serialization_password(&self) -> &str {
        self.serialization_password.as_str()
    }
}