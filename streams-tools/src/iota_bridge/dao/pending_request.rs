use anyhow::Result;

use serde::{
    Deserialize,
    Serialize
};

use rusqlite::{
    Connection,
    params,
};

use crate::{
    helpers::SerializationCallbackRefToClosureI64,
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
    }
};

pub type MsgIdTransferType = Vec<u8>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct PendingRequest {
    pub dev_eui: String,
    pub msg_id: MsgIdTransferType,
    pub initialization_cnt: u8,
    pub streams_api_request: Vec<u8>,
    pub request_key: Option<i64>
}

impl PendingRequest {
    pub fn new(dev_eui: String, msg_id: MsgIdTransferType, initialization_cnt: u8, streams_api_request: Vec<u8>) -> Self {
        PendingRequest {
            request_key: None,
            dev_eui,
            msg_id,
            initialization_cnt,
            streams_api_request,
        }
    }
}

pub struct PendingRequestDaoManager {
    connection: Connection,
    options: DbFileBasedDaoManagerOptions,
}

impl Clone for PendingRequestDaoManager {
    fn clone(&self) -> Self {
        PendingRequestDaoManager{
            connection: self.options.get_new_connection(),
            options: self.options.clone(),
        }
    }
}

impl DaoManager for PendingRequestDaoManager {
    type ItemType = PendingRequest;
    type PrimaryKeyType = i64;
    type SerializationCallbackType = SerializationCallbackRefToClosureI64;
    type OptionsType = DbFileBasedDaoManagerOptions;

    const ITEM_TYPE_NAME: &'static str = "PendingRequest";
    const DAO_MANAGER_NAME: &'static str = "PendingRequestDaoManager";
    const PRIMARY_KEY_COLUMN_NAME: &'static str = "request_key";
    const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

    fn new(options: DbFileBasedDaoManagerOptions) -> Self {
        PendingRequestDaoManager{
            connection: options.get_new_connection(),
            options,
        }
    }

    fn get_connection(&self) -> &Connection {
        &self.connection
    }

    fn get_table_name(&self) -> String { "pending_request".to_string() }

    fn update_db_schema_to_current_version(&self) -> Result<()> {
        update_db_schema_to_current_version(self)
    }

    fn init_db_schema(&self) -> Result<()> {
        self.connection.execute(format!("CREATE TABLE {} (\
                {} INTEGER NOT NULL PRIMARY KEY,\
                dev_eui TEXT NOT NULL,\
                msg_id BLOB NOT NULL,\
                initialization_cnt INTEGER NOT NULL,\
                streams_api_request BLOB NOT NULL\
            )
            ", self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
            .expect("Error on executing 'CREATE TABLE' for PendingRequest");
        Ok(())
    }

    fn get_item_from_db(&self, request_key: &Self::PrimaryKeyType) -> Result<PendingRequest> {
        get_item_from_db(self, request_key, MatchType::ExactMatch)
    }

    fn search_item(&self, _request_key_starts_with: &str) -> Result<PendingRequest>{
        unimplemented!()
    }

    fn find_all(&self, _request_key_starts_with: &str, _limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        unimplemented!()
    }

    fn filter(&self, conditions: Vec<Condition>, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
        unimplemented!()
    }

    fn write_item_to_db(&self, item: &PendingRequest) -> Result<Self::PrimaryKeyType> {
        let _rows = if let Some(request_key) = item.request_key {
            self.connection.execute(format!(
                "INSERT OR REPLACE INTO {} ({}, dev_eui, msg_id, initialization_cnt, streams_api_request) VALUES (?, ?, ?, ?, ?)",
                    self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(),
                params![
                    &request_key,
                    &item.dev_eui,
                    &item.msg_id,
                    &item.initialization_cnt,
                    &item.streams_api_request,
            ]).unwrap()
        } else {
            self.connection.execute(format!(
                "INSERT OR REPLACE INTO {} (dev_eui, msg_id, initialization_cnt, streams_api_request) VALUES (?, ?, ?, ?)", self.get_table_name()).as_str(),
                params![
                    &item.dev_eui,
                    &item.msg_id,
                    &item.initialization_cnt,
                    &item.streams_api_request,
            ]).unwrap()
        };

        Ok(item.request_key.unwrap_or(self.connection.last_insert_rowid()))
    }

    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType {
        let options = self.options.clone();
        let dev_eui = item.dev_eui.clone();
        let msg_id = item.msg_id.clone();
        let initialization_cnt = item.initialization_cnt;

        Box::new( move |request_id: Self::PrimaryKeyType, streams_api_request: Vec<u8>| -> Result<usize> {
            let ret_val = streams_api_request.len();
            let new_pending_req = PendingRequest {
                request_key: Some(request_id),
                dev_eui,
                msg_id,
                initialization_cnt,
                streams_api_request,
            };
            let this = PendingRequestDaoManager::new(options);
            this.write_item_to_db(&new_pending_req)?;
            Ok(ret_val)
        })
    }


    fn delete_item_in_db(&self, key: &Self::PrimaryKeyType) -> Result<()> {
        let _rows = self.connection.execute(
            format!(
                "DELETE FROM {} WHERE {} = {}",
                self.get_table_name(),
                Self::PRIMARY_KEY_COLUMN_NAME,
                key
            ).as_str(),
            params![]
        ).unwrap();
        Ok(())
    }
}

pub type PendingRequestDataStore = DaoDataStore<PendingRequestDaoManager>;

// These tests need to be started as follows:
//      > cargo test --package streams-tools --lib iota_bridge::dao::pending_request::tests  --features iota_bridge
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::SerializationCallbackRefToClosureI64;
    use iota_streams::app::transport::tangle::MSGID_SIZE;

    const DEV_EUI: &str = "12345678";
    const MSG_ID: [u8; MSGID_SIZE] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    const INITIALIZATION_CNT: u8 = 0;

    #[test]
    fn test_pending_request_dao_manager() {
        let connection = Rc::new(Connection::open_in_memory().unwrap());
        let dao_manager = PendingRequestDaoManager::new(connection.clone());
        dao_manager.init_db_schema().unwrap();

        let mut pending_request = PendingRequest::new(
            DEV_EUI.to_string(),
            MSG_ID.to_vec(),
            INITIALIZATION_CNT,
            vec![1, 2, 3]
        );
        let request_key = dao_manager.write_item_to_db(&pending_request).unwrap();
        pending_request.request_key = Some(request_key);

        let pending_request_from_db = dao_manager.get_item_from_db(&request_key).unwrap();
        assert_eq!(pending_request, pending_request_from_db);

        dao_manager.delete_item_in_db(&request_key).unwrap();
        match dao_manager.get_item_from_db(&request_key) {
            Ok(item) => {
                assert_eq!(item.dev_eui.as_str(), "Should no more exist in db")
            }
            Err(_) => {
                // Everything is fine
            }
        }
    }

    fn create_data_store_with_item_0() -> (PendingRequestDaoManager, PendingRequestDataStore, PendingRequest, SerializationCallbackRefToClosureI64) {
        let connection = Rc::new(Connection::open_in_memory().unwrap());
        let dao_manager = PendingRequestDaoManager::new(connection.clone());
        dao_manager.init_db_schema().unwrap();

        let mut pending_request =  PendingRequest::new(
            DEV_EUI.to_string(),
            MSG_ID.to_vec(),
            INITIALIZATION_CNT,
            vec![1, 2, 3]
        );

        let request_id = dao_manager.write_item_to_db(&pending_request).unwrap();
        pending_request.request_key = Some(request_id);

        let data_store = PendingRequestDataStore::new_from_connection(
            dao_manager.connection.clone(),
            None
        );

        let (pending_request_from_db, serialization_callback) = data_store.get_item(&request_id).unwrap();
        assert_eq!(pending_request_from_db, pending_request);
        (dao_manager, data_store, pending_request, serialization_callback)
    }

    #[test]
    fn test_pending_request_serialization_callback() {
        let (pending_request_dao_manager,
            _data_store,
            mut pending_request_0,
            serialization_callback,
        ) = create_data_store_with_item_0();

        // test_item_0.some_data is originally vec![1, 2, 3, 4]
        // adding some more data here
        pending_request_0.streams_api_request.push(5);
        pending_request_0.streams_api_request.push(6);
        pending_request_0.streams_api_request.push(7);

        if let Some (request_key) = pending_request_0.request_key {
            serialization_callback(request_key, pending_request_0.streams_api_request.clone()).unwrap();

            let item_from_db_0 = pending_request_dao_manager.get_item_from_db(&request_key).unwrap();
            assert_eq!(item_from_db_0, pending_request_0);
        } else {
            assert_ne!(pending_request_0.request_key, None);
        }
    }
}