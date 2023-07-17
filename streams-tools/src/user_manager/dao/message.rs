use anyhow::Result;

use serde::{
    Deserialize,
    Serialize
};

use rusqlite::Connection;

use serde_rusqlite::to_params_named;

use crate::{
    helpers::SerializationCallbackRefToClosureString,
    dao_helpers::{
        DaoManager,
        DaoDataStore,
        DbSchemaVersionType,
        DbFileBasedDaoManagerOpt,
        Limit,
        MatchType,
        Condition,
        filter_items,
        get_item_from_db,
        find_all_items_in_db,
        update_db_schema_to_current_version,
    }
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Message {
    pub message_id: String,
    pub wrapped_binary: Vec<u8>,
}

#[derive(Clone)]
pub struct MessageDataStoreOptions {
    pub file_path_and_name: String,
    pub streams_channel_id: String,
}

impl DbFileBasedDaoManagerOpt for MessageDataStoreOptions {
    fn file_path_and_name(&self) -> String {
        self.file_path_and_name.clone()
    }
}


pub struct MessageDaoManager{
    connection: Connection,
    options: MessageDataStoreOptions,
}

impl Clone for MessageDaoManager {
    fn clone(&self) -> Self {
        MessageDaoManager{
            connection: self.options.get_new_connection(),
            options: self.options.clone(),
        }
    }
}

impl DaoManager for MessageDaoManager {
    type ItemType = Message;
    type PrimaryKeyType = String;
    type SerializationCallbackType = SerializationCallbackRefToClosureString;
    type OptionsType = MessageDataStoreOptions;

    const ITEM_TYPE_NAME: &'static str = "Message";
    const DAO_MANAGER_NAME: &'static str = "MessageDaoManager";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "message_id";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new(options: MessageDataStoreOptions) -> Self {
        MessageDaoManager{
            connection: options.get_new_connection(),
            options,
        }
    }

    fn get_table_name(&self) -> String {
        format!("message_{}", self.options.streams_channel_id)
    }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        update_db_schema_to_current_version(self)
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
            {} TEXT NOT NULL PRIMARY KEY,\
            wrapped_binary BLOB NOT NULL\
            )
            ", self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
            .expect("Error on executing 'CREATE TABLE' for Message");
        Ok(())
    }

    fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<Message> {
        get_item_from_db(self, key, MatchType::ExactMatch)
    }

    fn search_item(&self, channel_starts_with: &str) -> Result<Message>{
        get_item_from_db(self, &channel_starts_with.to_string(),  MatchType::StartsWith)
    }

    fn find_all(&self, channel_starts_with: &str, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        find_all_items_in_db(self, &channel_starts_with.to_string(), limit)
    }

    fn filter(&self, conditions: Vec<Condition>, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        filter_items(self, &conditions, limit)
    }

    fn write_item_to_db(&self, item: &Message) -> Result<Self::PrimaryKeyType> {
        let _rows = self.connection.execute(format!(
            "INSERT OR REPLACE INTO {} (message_id, wrapped_binary) VALUES (\
                                :message_id,\
                                :wrapped_binary\
            )", self.get_table_name()).as_str(),
                                           to_params_named(item).unwrap().to_slice().as_slice())
            .expect("Error on executing 'INSERT INTO' for Message");
        Ok(item.message_id.clone())
    }

    fn get_serialization_callback(&self, _item: &Self::ItemType) -> Self::SerializationCallbackType {
        let this = self.clone();
        Box::new( move |message_id: String, wrapped_binary: Vec<u8>| -> Result<usize> {
            let mut new_msg = Message::default();
            let ret_val = wrapped_binary.len();
            new_msg.wrapped_binary = wrapped_binary;
            new_msg.message_id = message_id;
            this.write_item_to_db(&new_msg)?;
            Ok(ret_val)
        })
    }

    fn delete_item_in_db(&self, _key: &Self::PrimaryKeyType) -> Result<()> {
        unimplemented!();
    }
}

unsafe impl Send for MessageDaoManager {}
unsafe impl Sync for MessageDaoManager {}

pub type MessageDataStore = DaoDataStore<MessageDaoManager>;
