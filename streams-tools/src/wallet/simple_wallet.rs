pub trait SimpleWallet {
    const IS_USABLE_WALLET: bool;

    fn new(file_path_name: &str) -> Self;
    fn get_seed(&self) -> &str;
    fn get_serialization_password(&self) -> &str;
}