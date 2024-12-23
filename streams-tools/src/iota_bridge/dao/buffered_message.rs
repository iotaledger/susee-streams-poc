use anyhow::Result;

use serde::{
    Deserialize,
    Serialize,
};

use rusqlite::{
    Connection,
    params,
};

use crate::{
    helpers::SerializationCallbackRefToClosureI64,
    binary_persist::LinkedMessage,
    dao_helpers::{
        DaoManager,
        DbSchemaVersionType,
        DaoDataStore,
        DbFileBasedDaoManagerOptions,
        DbFileBasedDaoManagerOpt,
        Limit,
        MatchType,
        Condition,
        get_item_from_db,
        update_db_schema_to_current_version,
        filter_items,
    }
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct BufferedMessage {
    pub id: Option<i64>,
    pub link: String,
    pub body: Vec<u8>,
}

impl BufferedMessage {
    pub fn new(message: LinkedMessage) -> Self {
        BufferedMessage {
            id: None,
            link: message.link.to_string(),
            body: message.body.into_body(),
        }
    }
}

pub struct BufferedMessageDaoManager {
    connection: Connection,
    options: DbFileBasedDaoManagerOptions,
}

impl Clone for BufferedMessageDaoManager {
    fn clone(&self) -> Self {
        BufferedMessageDaoManager{
            connection: self.options.get_new_connection(),
            options: self.options.clone(),
        }
    }
}

impl DaoManager for BufferedMessageDaoManager {
    type ItemType = BufferedMessage;
    type PrimaryKeyType = i64;
    type SerializationCallbackType = SerializationCallbackRefToClosureI64;
    type OptionsType = DbFileBasedDaoManagerOptions;

    const ITEM_TYPE_NAME: &'static str = "BufferedMessage";
    const DAO_MANAGER_NAME: &'static str = "BufferedMessageDaoManager";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "id";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new(options: DbFileBasedDaoManagerOptions) -> Self {
        BufferedMessageDaoManager{
            connection: options.get_new_connection(),
            options,
        }
    }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn get_table_name(&self) -> String { "buffered_message".to_string() }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        update_db_schema_to_current_version(self)
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
                {} INTEGER NOT NULL PRIMARY KEY,\
                link TEXT NOT NULL,\
                body BLOB NOT NULL\
            )
            ", self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
            .expect("Error on executing 'CREATE TABLE' for BufferedMessage");
        Ok(())
    }

    fn get_item_from_db(&self, id: &Self::PrimaryKeyType) -> Result<BufferedMessage> {
        get_item_from_db(self, id, MatchType::ExactMatch)
    }

    fn search_item(&self, _id_starts_with: &str) -> Result<BufferedMessage>{
        unimplemented!()
    }

    fn find_all(&self, _id_starts_with: &str, _limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        unimplemented!()
    }

    fn filter(&self, conditions: Vec<Condition>, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        filter_items(self, &conditions, limit)
    }

    fn write_item_to_db(&self, item: &BufferedMessage) -> Result<Self::PrimaryKeyType> {
        let _rows = if let Some(id) = item.id {
            self.connection.execute(format!(
                "INSERT OR REPLACE INTO {} ({}, link, body) VALUES (?, ?, ?)",
                self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(),
                                    params![
                    &id,
                    &item.link,
                    &item.body
            ]).unwrap()
        } else {
            self.connection.execute(format!(
                "INSERT OR REPLACE INTO {} (link, body) VALUES (?, ?)",
                self.get_table_name()).as_str(),
                                    params![
                    &item.link,
                    &item.body
            ]).unwrap()
        };

        Ok(item.id.unwrap_or(self.connection.last_insert_rowid()))
    }

    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType {
        let options = self.options.clone();
        let link = item.link.clone();
        Box::new( move |id: Self::PrimaryKeyType, body: Vec<u8>| -> Result<usize> {
            let ret_val = body.len();
            let new_msg = BufferedMessage {
                id: Some(id),
                link,
                body,
            };
            let this = BufferedMessageDaoManager::new(options);
            this.write_item_to_db(&new_msg)?;
            Ok(ret_val)
        })
    }

    fn delete_item_in_db(&self, id: &Self::PrimaryKeyType) -> Result<()> {
        let _rows = self.connection.execute(
            format!(
                "DELETE FROM {} WHERE {} = {}",
                self.get_table_name(),
                Self::PRIMARY_KEY_COLUMN_NAME,
                id
            ).as_str(),
            params![]
        ).unwrap();
        Ok(())
    }
}

pub type BufferedMessageDataStore = DaoDataStore<BufferedMessageDaoManager>;

// These tests need to be started as follows:
//      > cargo test --package streams-tools --lib iota_bridge::dao::buffered_message::tests  --features iota_bridge
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_helpers::get_linked_message,
    };

    #[test]
    fn test_buffered_message_dao_manager() {
        let options = DbFileBasedDaoManagerOptions { file_path_and_name: "not used".to_string() };
        let dao_manager = BufferedMessageDaoManager::new(options);
        dao_manager.init_db_schema().unwrap();

        let mut buffered_message = BufferedMessage::new(get_linked_message());
        let req_id = dao_manager.write_item_to_db(&buffered_message).unwrap();
        buffered_message.id = req_id;

        let buffered_message_from_db = dao_manager.get_item_from_db(&req_id).unwrap();
        assert_eq!(buffered_message, buffered_message_from_db);

        dao_manager.delete_item_in_db(&req_id).unwrap();
        match dao_manager.get_item_from_db(&req_id) {
            Ok(item) => {
                assert_eq!(item.id.to_string(), "Should no more exist in db")
            }
            Err(_) => {
                // Everything is fine
            }
        }
    }
}