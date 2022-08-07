pub mod simple_wallet;
pub mod dummy_wallet;
#[cfg(feature = "std")]
pub mod plain_text_wallet;

pub use {
    simple_wallet::SimpleWallet,
    dummy_wallet::DummyWallet,
    plain_text_wallet::*
};