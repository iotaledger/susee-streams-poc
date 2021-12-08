pub mod channel_manager;
pub mod subscriber_manager;

pub use {
    channel_manager::{
        ChannelManager,
        ChannelManagerPlainTextWallet,
        Author,
    },
    subscriber_manager::{
        SubscriberManager,
        SubscriberManagerPlainTextWallet
    }
};