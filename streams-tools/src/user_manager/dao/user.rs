use crate::{
    dao_helpers::{
        DaoManager,
        DbSchemaVersionType,
        get_item_from_db,
        update_db_schema_to_current_version,
    }
};

use anyhow::Result;

use serde::{
    Deserialize,
    Serialize
};
use rusqlite::{Connection};
use serde_rusqlite::to_params_named;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct User {
    pub streams_channel_id: String,
    pub streams_user_state: Vec<u8>,
    pub seed_derivation_phrase: String,
}

#[derive(Clone)]
pub struct UserDaoManager{
    connection: Rc<Connection>,
}

impl UserDaoManager {

    pub fn new_from_connection(connection: Rc<Connection>) -> Self {
        UserDaoManager {
            connection,
        }
    }
    pub fn search_user(&self, channel_starts_with: &str) -> Result<User>{
        get_item_from_db(self, channel_starts_with, Some(true))
    }

    pub fn get_user(&self, channel_id: &str) -> Result<User>{
        get_item_from_db(self, channel_id, None)
    }
}

impl DaoManager for UserDaoManager {
    type ItemType = User;
    const ITEM_TYPE_NAME: &'static str = "User";
    const TABLE_NAME: &'static str = "user";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "streams_channel_id";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        update_db_schema_to_current_version(self)
    }

    fn get_item_from_db(&self, key: &str) -> Result<User> {
        get_item_from_db(self, key, None)
    }

    fn write_item_to_db(&self, item: User) -> Result<usize> {
        let rows = self.connection.execute(format!(
            "INSERT INTO {} (streams_channel_id, streams_user_state, seed_derivation_phrase) VALUES (\
                                :streams_channel_id,\
                                :streams_user_state,\
                                :seed_derivation_phrase\
            )", Self::TABLE_NAME).as_str(),
                                           to_params_named(item).unwrap().to_slice().as_slice())
            .expect("Error on executing 'INSERT INTO' for User");
        Ok(rows)
    }

    fn update_item_in_db(&self, item: User) -> Result<usize> {
        let rows = self.connection.execute(format!(
            "UPDATE {} SET streams_user_state = :streams_user_state\
             WHERE streams_channel_id = ':streams_channel_id'\
             ", Self::TABLE_NAME).as_str(),
                                           to_params_named(item).unwrap().to_slice().as_slice())
            .expect("Error on executing 'UPDATE' for User");
        Ok(rows)
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
            streams_channel_id TEXT NOT NULL PRIMARY KEY,\
            streams_user_state BLOB NOT NULL,\
            seed_derivation_phrase TEXT NOT NULL\
            )
            ", Self::TABLE_NAME).as_str(), []).expect("Error on executing 'CREATE TABLE' for User");
        Ok(())
    }
}

