pub mod helpers;
pub mod client;
pub mod plain_text_wallet;
pub mod user_manager;
pub mod dummy_wallet;

pub use {
    user_manager::*,
    client::*,
    plain_text_wallet::{
        PlainTextWallet,
        SimpleWallet,
    },
    dummy_wallet::DummyWallet,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
