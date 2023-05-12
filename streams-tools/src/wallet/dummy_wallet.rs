use super::simple_wallet::SimpleWallet;

use anyhow::Result;

#[derive(Clone, Default)]
pub struct DummyWallet {
    initialization_cnt: u8
}

const SEED: &'static str = "--- This is a dummy seed used by a dummy wallet --- This is a dummy seed used ...";

const SEREALIZATION_PASSWD: &'static str = "dummy serialization password";

impl SimpleWallet for DummyWallet {
    const IS_USABLE_WALLET: bool = false;

    fn new(_file_path_name: &str) -> Self {
        Self::default()
    }

    fn get_seed(&self) -> &str {
        SEED
    }

    fn get_serialization_password(&self) -> &str {
        SEREALIZATION_PASSWD
    }

    fn get_initialization_cnt(&self) -> u8 {
        self.initialization_cnt
    }

    fn increment_initialization_cnt(&mut self) -> Result<u8> {
        self.initialization_cnt += 1;
        Ok(self.initialization_cnt)
    }
}


// These tests need to be started as follows:
//      > cargo test --package streams-tools --lib wallet::dummy_wallet::tests
//
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_wallet() {
        let wallet = DummyWallet::new("dummy_wallet");
        assert_eq!(wallet.get_seed(), SEED);
        assert_eq!(wallet.get_serialization_password(), SEREALIZATION_PASSWD);
    }
}