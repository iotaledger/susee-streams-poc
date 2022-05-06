pub trait SimpleWallet {
    const IS_USABLE_WALLET: bool;
    fn get_seed(&self) -> &str;
    fn get_serialization_password(&self) -> &str;
}