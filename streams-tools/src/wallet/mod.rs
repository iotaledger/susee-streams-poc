pub mod simple_wallet;
pub mod dummy_wallet;
pub mod plain_text_wallet;

pub use {
    simple_wallet::SimpleWallet,
    dummy_wallet::DummyWallet,
};

pub use {
    plain_text_wallet::*
};