use crate::dao_helpers::{
    DaoManager,
    DbSchemaVersionType,
    DaoDataStore,
    get_item_from_db,
    update_db_schema_to_current_version,
};

use serde::{
    Deserialize,
    Serialize
};
use rusqlite::Connection;
use serde_rusqlite::to_params_named;

use anyhow::Result;
use std::rc::Rc;
use crate::helpers::SerializationCallbackRefToClosure;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct LoraWanNode {
    pub dev_eui: String,
    pub streams_channel_id: String,
}

#[derive(Clone)]
pub struct LoraWanNodeDaoManager {
    connection: Rc<Connection>,
}

impl DaoManager for LoraWanNodeDaoManager {
    type ItemType = LoraWanNode;
    type SerializationCallbackType = SerializationCallbackRefToClosure;
    const ITEM_TYPE_NAME: &'static str = "LoraWanNode";
    const DAO_MANAGER_NAME: &'static str = "LoraWanNodeDaoManager";
    const TABLE_NAME: &'static str = "lora_wan_node";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "dev_eui";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new_from_connection(connection: Rc<Connection>) -> Self {
        LoraWanNodeDaoManager {
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
            dev_eui TEXT NOT NULL PRIMARY KEY,\
            streams_channel_id  TEXT NOT NULL\
            )
            ", Self::TABLE_NAME).as_str(), []).expect("Error on executing 'CREATE TABLE' for LoraWanNode");

        self.connection.execute("CREATE INDEX idx_sensor_streams_channel_id ON sensor", [])
            .expect("Error on executing 'CREATE INDEX' for LoraWanNode");
        Ok(())
    }

    fn get_item_from_db(&self, key: &str) -> Result<LoraWanNode> {
        get_item_from_db(self, key, None)
    }

    fn search_item(&self, channel_starts_with: &str) -> Result<LoraWanNode>{
        get_item_from_db(self, channel_starts_with, Some(true))
    }

    fn write_item_to_db(&self, item: LoraWanNode) -> Result<usize> {
        let rows = self.connection.execute(format!(
            "INSERT INTO {} (dev_eui, streams_channel_id) VALUES (:dev_eui, :streams_channel_id)", Self::TABLE_NAME).as_str(),
                           to_params_named(item).unwrap().to_slice().as_slice()).unwrap();
        Ok(rows)
    }

    fn update_item_in_db(&self, _item: LoraWanNode) -> Result<usize> {
        // Currently there is no need to update a lora_wan_node
        unimplemented!()
    }

    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType {
        let this = self.clone();
        let dev_eui = item.dev_eui.clone();
        Box::new( move |dev_eui: String, streams_channel_id_utf8_bytes: Vec<u8>| -> Result<usize> {
            let new_node = LoraWanNode {
                dev_eui,
                streams_channel_id: String::from_utf8(streams_channel_id_utf8_bytes)
                    .expect("Error while reading streams_channel_id_utf8_bytes into String instance.")
            };
            this.write_item_to_db(new_node)
        })
    }
}

pub type LoraWanNodeDataStore = DaoDataStore<LoraWanNodeDaoManager>;