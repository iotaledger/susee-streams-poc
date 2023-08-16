pub mod subscriber_manager;
pub mod compressed_state;

#[cfg(feature = "std")]
pub mod channel_manager;
#[cfg(feature = "dao")]
pub mod multi_channel_management;
#[cfg(feature = "std")]
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
    },
    subscriber_manager::{
        SubscriberManagerPlainTextWallet
    },
    message_manager::{
        MessageManager,
    }
};

#[cfg(feature = "dao")]
pub use dao::user::{
    UserDataStore
};

// This is a placeholder struct that needs to be replaced with an indexer that uses a
// inx-streams-collector to retrieve messages from the tangle.
//
// The inx-streams-collector will be an extended version of the inx-collector, implemented
// as a for of the original Teleconsys repository.
//
// TODO: Implement inx-streams-collector
pub struct DummyMsgIndexer {}

#[cfg(feature = "std")]
use lets::{
    transport::tangle::MessageIndex,
    message::TransportMessage
};
#[cfg(feature = "std")]
use async_trait::async_trait;

#[cfg(feature = "std")]
#[async_trait(?Send)]
impl MessageIndex for DummyMsgIndexer {
    async fn get_message_by_tag(&self, msg_index: [u8; 32]) -> lets::error::Result<Vec<TransportMessage>> {
        println!("[DummyMsgIndexer - get_message_by_tag()] Request for msg_index {}", hex::encode(msg_index));
        Ok(vec![TransportMessage::new(Vec::default())])
    }
}