use anyhow::Result;
use futures::TryStreamExt;
use lets::message::TransportMessage;

use streams::{
    User,
    transport::Transport,
};

use crate::{
    dao_helpers::Limit,
};

use super::dao::message::{
    MessageDataStore,
    MessageDataStoreOptions,
    Message as DaoMessage
};

pub struct MessageManager<'a, TransT> {
    user: &'a mut User<TransT>,
    message_data_store: MessageDataStore,
    streams_channel_id: String,
}

impl<'a, TransT> MessageManager<'a, TransT> {
    pub fn new(user: &'a mut User<TransT>, channel_id: String, db_file_name: String) -> Self {
        let msg_data_store_opt = MessageDataStoreOptions {
            file_path_and_name: db_file_name,
            streams_channel_id: channel_id.clone()
        };
        log::debug!("[fn new()] Creating new MessageManager using MessageDataStoreOptions {}", msg_data_store_opt);
        let message_data_store = MessageDataStore::new(msg_data_store_opt);
        MessageManager {
            user,
            message_data_store,
            streams_channel_id: channel_id,
        }
    }

    pub fn index(&self, limit: Option<Limit>) -> Result<(Vec<DaoMessage>, usize)> {
        self.message_data_store.find_all("", limit)
    }

    pub fn get(&self, message_id: &str) -> Result<DaoMessage> {
        self.message_data_store.get_item_read_only(&message_id.to_string())
    }
}

impl<'a, TransT> MessageManager<'a, TransT>
where
    TransT: for<'b> Transport<'b, Msg = TransportMessage>
{
    pub async fn sync(&mut self) -> Result<u32> {
        let mut messages = self.user.messages();

        let mut num_messages_stored = 0;
        log::debug!("[fn sync()] Starting to sync addresses for channel {}", self.streams_channel_id);
        while let Some(msg) = messages.try_next().await? {
            num_messages_stored += 1;
            log::debug!("[fn sync()] Fetched message {} to trigger MessageIndexer message caching", msg.address.relative().to_string());
        }
        log::info!("[fn sync()] Fetched {} messages to trigger MessageIndexer message caching", num_messages_stored);
        Ok(num_messages_stored)
    }
}