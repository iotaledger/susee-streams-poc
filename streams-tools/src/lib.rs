#![feature(generic_const_exprs)]
#![feature(hasher_prefixfree_extras)]

pub mod wallet;
pub mod user_manager;
pub mod client;
pub mod http;
pub mod binary_persist;
pub mod remote;

#[cfg(feature = "std")]
pub mod helpers;
#[cfg(feature = "iota_bridge")]
pub mod iota_bridge;

#[cfg(feature = "dao")]
pub mod dao_helpers;


pub use {
    wallet::{
        dummy_wallet::DummyWallet,
        simple_wallet::SimpleWallet,
    },
    client::*,
    user_manager::*,
};

pub use {
    wallet::plain_text_wallet::{
        PlainTextWallet,
    },
};

#[cfg(feature = "iota_bridge")]
pub use {
    iota_bridge::IotaBridge,
};
