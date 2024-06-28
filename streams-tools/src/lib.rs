#![feature(generic_const_exprs)]
#![feature(hasher_prefixfree_extras)]

pub mod wallet;
/// cbindgen:ignore
pub mod user_manager;
pub mod streams_transport;
pub mod http;
pub mod binary_persist;
pub mod remote;
pub mod lorawan_rest_helpers;

#[cfg(feature = "std")]
pub mod helpers;
#[cfg(feature = "iota_bridge")]
pub mod iota_bridge;

#[cfg(feature = "dao")]
pub mod dao_helpers;

#[cfg(feature = "explorer")]
pub mod explorer;

#[cfg(test)]
pub (crate) mod test_helpers;

#[cfg(any(feature = "explorer", feature = "iota_bridge"))]
pub mod threading_helpers;

pub use {
    wallet::{
        dummy_wallet::DummyWallet,
        simple_wallet::SimpleWallet,
        plain_text_wallet::{
            PlainTextWallet,
        },
    },
    streams_transport::*,
    user_manager::*,
    lorawan_rest_helpers::*,
};

#[cfg(feature = "iota_bridge")]
pub use {
    iota_bridge::IotaBridge,
};
