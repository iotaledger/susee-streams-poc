#![feature(generic_const_exprs)]

pub mod wallet;
pub mod user_manager;
pub mod client;

#[cfg(feature = "std")]
pub mod helpers;

pub use {
    wallet::{
        dummy_wallet::DummyWallet,
        simple_wallet::SimpleWallet,
    },
    client::*,
    user_manager::*,
};

#[cfg(feature = "std")]
pub use {
    wallet::plain_text_wallet::{
        PlainTextWallet,
    },
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
