use anyhow::Result;

pub trait SimpleWallet {
    const IS_USABLE_WALLET: bool;

    fn new(file_path_name: &str) -> Self;
    fn get_seed(&self) -> &str;
    fn get_serialization_password(&self) -> &str;

    fn get_initialization_cnt(&self) -> u8;
    fn increment_initialization_cnt(&mut self) -> Result<u8>;
}