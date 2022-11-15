use crate::dao_helpers::{DaoManager, DbSchemaVersionType, get_item_from_db};
use serde::{
    Deserialize,
    Serialize
};
use rusqlite::Connection;
use serde_rusqlite::to_params_named;

use anyhow::Result;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Client {
    dev_eui: String,
    streams_channel_id: String,
}

#[derive(Clone)]
pub struct ClientDaoManager {
    connection: Rc<Connection>,
}

impl DaoManager for ClientDaoManager {
    type ItemType = Client;
    const ITEM_TYPE_NAME: &'static str = "Client";
    const TABLE_NAME: &'static str = "client";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "dev_eui";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        Ok(())
    }

    fn get_item_from_db(&self, key: &str) -> Result<Client> {
        get_item_from_db(self, key, None)
    }

    fn write_item_to_db(&self, item: Client) -> Result<usize> {
        let rows = self.connection.execute(format!(
            "INSERT INTO {} (dev_eui, streams_channel_id) VALUES (:dev_eui, :streams_channel_id)", Self::TABLE_NAME).as_str(),
                           to_params_named(item).unwrap().to_slice().as_slice()).unwrap();
        Ok(rows)
    }

    fn update_item_in_db(&self, _item: Client) -> Result<usize> {
        // Currently there is no need to update a client
        unimplemented!()
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
            dev_eui TEXT NOT NULL PRIMARY KEY,\
            streams_channel_id  TEXT NOT NULL\
            )
            ", Self::TABLE_NAME).as_str(), []).expect("Error on executing 'CREATE TABLE' for Client");

        self.connection.execute("CREATE INDEX idx_sensor_streams_channel_id ON sensor", [])
            .expect("Error on executing 'CREATE INDEX' for Client");
        Ok(())
    }
}
