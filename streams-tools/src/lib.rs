#![feature(generic_const_exprs)]

pub mod wallet;
pub mod user_manager;
pub mod client;
pub mod http;
pub mod binary_persist;

#[cfg(feature = "std")]
pub mod remote;
#[cfg(feature = "std")]
pub mod helpers;
#[cfg(feature = "std")]
pub mod iota_bridge;

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
    iota_bridge::IotaBridge,
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
