use crate::{
    helpers::SerializationCallbackRefToClosureString,
    dao_helpers::{
        DaoManager,
        DaoDataStore,
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

use rusqlite::Connection;
use serde_rusqlite::to_params_named;
use std::rc::Rc;


#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct User {
    pub streams_channel_id: String,
    pub streams_user_state: Vec<u8>,
    pub seed_derivation_phrase: String,
}

#[derive(Clone)]
pub struct UserDaoManager{
    connection: Rc<Connection>,
}

impl DaoManager for UserDaoManager {
    type ItemType = User;
    type PrimaryKeyType = String;
    type SerializationCallbackType = SerializationCallbackRefToClosureString;

    const ITEM_TYPE_NAME: &'static str = "User";
    const DAO_MANAGER_NAME: &'static str = "UserDaoManager";
    const TABLE_NAME: &'static str = "user";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "streams_channel_id";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new_from_connection(connection: Rc<Connection>) -> Self {
        UserDaoManager {
            connection,
        }
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
            streams_user_state BLOB NOT NULL,\
            seed_derivation_phrase TEXT NOT NULL\
            )
            ", Self::TABLE_NAME, Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
            .expect("Error on executing 'CREATE TABLE' for User");
        Ok(())
    }

    fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<User> {
        get_item_from_db(self, key, None)
    }

    fn search_item(&self, channel_starts_with: &str) -> Result<User>{
        get_item_from_db(self, &channel_starts_with.to_string(), Some(true))
    }

    fn write_item_to_db(&self, item: &User) -> Result<Self::PrimaryKeyType> {
        let _rows = self.connection.execute(format!(
            "INSERT INTO {} (streams_channel_id, streams_user_state, seed_derivation_phrase) VALUES (\
                                :streams_channel_id,\
                                :streams_user_state,\
                                :seed_derivation_phrase\
            )", Self::TABLE_NAME).as_str(),
                                           to_params_named(item).unwrap().to_slice().as_slice())
            .expect("Error on executing 'INSERT INTO' for User");
        Ok(item.streams_channel_id.clone())
    }

    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType {
        let this = self.clone();
        let seed_derive_phrase = item.seed_derivation_phrase.clone();
        Box::new( move |streams_channel_id: String, user_state: Vec<u8>| -> Result<usize> {
            let mut new_user = User::default();
            let ret_val = user_state.len();
            new_user.streams_user_state = user_state;
            new_user.streams_channel_id = streams_channel_id;
            new_user.seed_derivation_phrase = seed_derive_phrase.clone();
            this.write_item_to_db(&new_user)?;
            Ok(ret_val)
        })
    }

    fn delete_item_in_db(&self, _key: &Self::PrimaryKeyType) -> Result<()> {
        unimplemented!();
    }
}

pub type UserDataStore = DaoDataStore<UserDaoManager>;