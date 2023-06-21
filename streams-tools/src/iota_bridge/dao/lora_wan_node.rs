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
        DbSchemaVersionType,
        DaoDataStore,
        DbFileBasedDaoManagerOpt,
        DbFileBasedDaoManagerOptions,
        get_item_from_db,
        find_all_items_in_db,
        update_db_schema_to_current_version,
    }
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct LoraWanNode {
    pub dev_eui: String,
    pub initialization_cnt: u8,
    pub streams_channel_id: String,
}

pub struct LoraWanNodeDaoManager {
    connection: Connection,
    options: DbFileBasedDaoManagerOptions,
}

impl Clone for LoraWanNodeDaoManager {
    fn clone(&self) -> Self {
        LoraWanNodeDaoManager{
            connection: self.options.get_new_connection(),
            options: self.options.clone(),
        }
    }
}

impl DaoManager for LoraWanNodeDaoManager {
    type ItemType = LoraWanNode;
    type PrimaryKeyType = String;
    type SerializationCallbackType = SerializationCallbackRefToClosureString;
    type OptionsType = DbFileBasedDaoManagerOptions;

    const ITEM_TYPE_NAME: &'static str = "LoraWanNode";
    const DAO_MANAGER_NAME: &'static str = "LoraWanNodeDaoManager";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "dev_eui";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new(options: DbFileBasedDaoManagerOptions) -> Self {
        LoraWanNodeDaoManager{
            connection: options.get_new_connection(),
            options,
        }
    }

    fn get_table_name(&self) -> String { "lora_wan_node".to_string() }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        update_db_schema_to_current_version(self)
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
                {} TEXT NOT NULL PRIMARY KEY,\
                initialization_cnt INTEGER NOT NULL,\
                streams_channel_id  TEXT NOT NULL\
            )
            ", self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
            .expect("Error on executing 'CREATE TABLE' for LoraWanNode");

        self.connection.execute(format!(
            "CREATE INDEX idx_{0}_streams_channel_id ON {0} (\
                streams_channel_id\
             )",
            self.get_table_name()).as_str(), [])
            .expect("Error on executing 'CREATE INDEX' for LoraWanNode");
        Ok(())
    }

    fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<LoraWanNode> {
        get_item_from_db(self, key, None)
    }

    fn search_item(&self, dev_eui_with: &str) -> Result<LoraWanNode>{
        get_item_from_db(self, &dev_eui_with.to_string(), Some(true))
    }

    fn find_all(&self, dev_eui_with: &str) -> Result<Vec<Self::ItemType>> {
        find_all_items_in_db(self, &dev_eui_with.to_string())
    }

    fn write_item_to_db(&self, item: &LoraWanNode) -> Result<Self::PrimaryKeyType> {
        let _rows = self.connection.execute(format!(
            "INSERT OR REPLACE INTO {} (dev_eui, initialization_cnt, streams_channel_id) \
            VALUES (:dev_eui, :initialization_cnt, :streams_channel_id)", self.get_table_name()).as_str(),
                           to_params_named(item).unwrap().to_slice().as_slice()).unwrap();
        Ok(item.dev_eui.clone())
    }

    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType {
        let options = self.options.clone();
        let initialization_cnt = item.initialization_cnt;
        Box::new( move |dev_eui: String, streams_channel_id_utf8_bytes: Vec<u8>| -> Result<usize> {
            let ret_val = streams_channel_id_utf8_bytes.len();
            let new_node = LoraWanNode {
                dev_eui,
                initialization_cnt,
                streams_channel_id: String::from_utf8(streams_channel_id_utf8_bytes)
                    .expect("Error while reading streams_channel_id_utf8_bytes into String instance.")
            };
            let this = LoraWanNodeDaoManager::new(options);
            this.write_item_to_db(&new_node)?;
            Ok(ret_val)
        })
    }

    fn delete_item_in_db(&self, _key: &Self::PrimaryKeyType) -> Result<()> {
        unimplemented!();
    }
}

pub type LoraWanNodeDataStore = DaoDataStore<LoraWanNodeDaoManager>;