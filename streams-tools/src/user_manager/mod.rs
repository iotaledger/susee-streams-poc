pub mod subscriber_manager;
pub mod compressed_state;

#[cfg(feature = "std")]
pub mod channel_manager;
#[cfg(feature = "dao")]
pub mod multi_channel_management;
#[cfg(feature = "dao")]
pub mod message_manager;


#[cfg(feature = "dao")]
pub mod dao;

pub use {
    subscriber_manager::{
        SubscriberManager,
    }
};

#[cfg(feature = "std")]
pub use {
    channel_manager::{
        ChannelManager,
        ChannelManagerPlainTextWallet,
        Author,
    },
    subscriber_manager::{
        SubscriberManagerPlainTextWallet
    }
};

#[cfg(feature = "dao")]
pub use {
    message_manager::{
        MessageManager,
    },
    dao::user::{
        UserDataStore
    }
};