use anyhow::Result;

use iota_streams::{
    app_channels::api::tangle::{
        Address,
        Message,
        IntoMessages,
        futures::TryStreamExt,
    },
    app::transport::{
        Transport,
    }
};

use crate::dao::message::{MessageDataStore, MessageDataStoreOptions, Message as DaoMessage};

pub struct MessageManager<'a,Trans: Transport<Address, Message> + Clone> {
    source: &'a mut dyn IntoMessages<Trans>,
    message_data_store: MessageDataStore,
}

impl<'a,Trans: Transport<Address, Message> + Clone> MessageManager<'a,Trans> {
    pub fn new(message_source: &'a mut dyn IntoMessages<Trans>, channel_id: String, db_file_name: String) -> Self {
        let message_data_store = MessageDataStore::new(MessageDataStoreOptions {
            file_path_and_name: db_file_name,
            streams_channel_id: channel_id.clone()
        });
        MessageManager{
            source: message_source,
            message_data_store,
        }
    }

    pub async fn sync(&mut self) -> Result<u32> {
        let mut messages = self.source.messages();
        let mut num_messages_stored = 0;
        while let Some(msg) = messages.try_next().await? {
            num_messages_stored += 1;
            self.message_data_store.write_item_to_db(
                &DaoMessage{
                    message_id: msg.link.msgid.to_string(),
                    wrapped_binary: vec![]
            })?;
        }
        Ok(num_messages_stored)
    }

    pub fn index(&self) -> Result<Vec<DaoMessage>> {
        self.message_data_store.find_all("")
    }

    pub fn get(&self, message_id: &str) -> Result<DaoMessage> {
        self.message_data_store.get_item_read_only(&message_id.to_string())
    }
}