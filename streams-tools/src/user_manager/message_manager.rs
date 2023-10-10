use anyhow::Result;
use futures::TryStreamExt;
use lets::message::TransportMessage;

use streams::{
    User,
    transport::Transport,
};

use crate::{
    dao_helpers::Limit,
    dao::message::{
        MessageDataStore,
        MessageDataStoreOptions,
        Message as DaoMessage
    }
};

pub struct MessageManager<'a, TransT> {
    user: &'a mut User<TransT>,
    message_data_store: MessageDataStore,
}

impl<'a, TransT> MessageManager<'a, TransT> {
    pub fn new(user: &'a mut User<TransT>, channel_id: String, db_file_name: String) -> Self {
        let message_data_store = MessageDataStore::new(MessageDataStoreOptions {
            file_path_and_name: db_file_name,
            streams_channel_id: channel_id.clone()
        });
        MessageManager {
            user,
            message_data_store,
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
        while let Some(msg) = messages.try_next().await? {
            num_messages_stored += 1;
            self.message_data_store.write_item_to_db(
                &DaoMessage{
                    message_id: msg.address.relative().to_string(),
                    wrapped_binary: vec![]
            })?;
        }
        Ok(num_messages_stored)
    }
}