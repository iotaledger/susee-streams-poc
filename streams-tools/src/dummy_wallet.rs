use crate::SimpleWallet;

pub struct DummyWallet {}

impl SimpleWallet for DummyWallet {
    const IS_USABLE_WALLET: bool = false;

    fn get_seed(&self) -> &str {
        "--- This is a dummy seed used by a dummy wallet --- This is a dummy seed used ..."
    }

    fn get_serialization_password(&self) -> &str {
        unimplemented!()
    }
}