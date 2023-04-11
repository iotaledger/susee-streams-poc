use super::simple_wallet::SimpleWallet;

#[derive(Clone, Default)]
pub struct DummyWallet {}

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
}

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