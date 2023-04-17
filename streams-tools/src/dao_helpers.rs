use std::{
    fmt::Display,
    rc::Rc,
};

use rusqlite::{
    Connection,
    Rows,
};

use serde_rusqlite::from_rows_ref;
use serde::de::DeserializeOwned;

use anyhow::{
    Result,
    bail,
};

use fallible_streaming_iterator::FallibleStreamingIterator;

pub type DbSchemaVersionType = i32;

pub trait DaoManager {

    type ItemType: DeserializeOwned;
    type PrimaryKeyType: Display;
    type SerializationCallbackType;

    const ITEM_TYPE_NAME: &'static str;
    const DAO_MANAGER_NAME: &'static str;
    const TABLE_NAME: &'static str;
    const PRIMARY_KEY_COLUMN_NAME: &'static str;
    const DB_SCHEMA_VERSION: DbSchemaVersionType;

    fn new_from_connection(connection: Rc<Connection>) -> Self;

    fn get_connection(&self) -> &Connection;

    fn update_db_schema_to_current_version(&self) -> Result<()>;

    fn init_db_schema(&self) -> Result<()>;

    fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<Self::ItemType>;

    fn search_item(&self, channel_starts_with: &str) -> Result<Self::ItemType>;

    fn write_item_to_db(&self, item: &Self::ItemType) -> Result<Self::PrimaryKeyType>;

    fn update_item_in_db(&self, item: &Self::ItemType) -> Result<usize>;

    fn delete_item_in_db(&self, key: &Self::PrimaryKeyType) -> Result<()>;

    // Provides a callback for serializing the item into the database.
    // This is needed by structs that manage the item after it has been initially created or
    // has ben read out of the database. After the manged item has been updated, the callback
    // is used by the managing struct to update the database.
    // The callback is usually a closure that can access specific fields of the item that have
    // been cloned into the closure when get_serialization_callback() is called.
    // The closure may often have the signature of the SerializationCallbackRefToClosure (String | I64) type
    //              move |id_goes_here: String, data_to_store: Vec<u8>| -> Result<usize>
    fn get_serialization_callback(&self, item: &Self::ItemType) -> Self::SerializationCallbackType;
}

pub fn get_item_from_db<'de, DaoManagerT: DaoManager>(
    dao_manager: &DaoManagerT,
    primary_key: &DaoManagerT::PrimaryKeyType,
    starts_with: Option<bool>
) -> Result<DaoManagerT::ItemType>
{
    let statement_str = build_query_statement(dao_manager, primary_key, starts_with)?;
    let mut statement = dao_manager.get_connection().prepare(statement_str.as_str())
        .expect(format!("Error on preparing 'SELECT * FROM' for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    let rows = statement.query([])
        .expect(format!("Error on querying prepared 'SELECT * FROM' statement for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    get_item_from_single_row_rowset(dao_manager, primary_key, rows, statement_str)
}

fn get_item_from_single_row_rowset<'de, DaoManagerT: DaoManager>(
    _dao_manager: &DaoManagerT,
    key: &DaoManagerT::PrimaryKeyType,
    mut rows: Rows, statement_str: String
) -> Result<DaoManagerT::ItemType>
{
    let mut res = from_rows_ref::<DaoManagerT::ItemType>(&mut rows);
    if let Some(item) = res.next() {
        let ret_val = item
            .expect(format!("Error on unwrapping next {} from_rows_ref",
                            DaoManagerT::ITEM_TYPE_NAME).as_str());

        if let Some(_additional_item) = res.next() {
            bail!("Found more than one {} in table '{}' having a matching '{}' column value starting with '{}'\n\
                   Used SQL statement: {}",
                DaoManagerT::ITEM_TYPE_NAME,
                DaoManagerT::TABLE_NAME,
                DaoManagerT::PRIMARY_KEY_COLUMN_NAME,
                key,
                statement_str,
            )
        }
        Ok(ret_val)
    } else {
        bail!("Could not find any {} in table '{}' for column '{}' with key '{}'\n\
        Used SQL statement: {}",
            DaoManagerT::ITEM_TYPE_NAME,
            DaoManagerT::TABLE_NAME,
            DaoManagerT::PRIMARY_KEY_COLUMN_NAME,
            key,
            statement_str,
        )
    }
}

fn build_query_statement<DaoManagerT: DaoManager>(
    _dao_manager: &DaoManagerT,
    primary_key: &DaoManagerT::PrimaryKeyType,
    starts_with: Option<bool>
) -> Result<String>
{
    let mut operator = "=";
    let mut wildcard = "";
    if let Some(starts_with) = starts_with {
        if starts_with {
            operator = "LIKE";
            wildcard = "%";
        }
    }
    let statement = format!("SELECT * FROM {} WHERE {} {} '{}{}'",
                            DaoManagerT::TABLE_NAME,
                            DaoManagerT::PRIMARY_KEY_COLUMN_NAME,

                            operator,
                            primary_key,
                            wildcard,
    );
    Ok(statement)
}

pub fn get_schema_version_in_database<DaoManagerT: DaoManager>(dao_manager: &DaoManagerT) -> Result<DbSchemaVersionType> {
    // Currently we do not manage updates of db schemas. We are using only the initial
    // db schema. Therefore we only check if the table needed for our data
    // already exists. If the table exists we'll return DB_SCHEMA_VERSION otherwise 0.
    // To use this code in production the version of the database schema
    // must be incremented every time the schema is changed and the schema version
    // must be stored in an additional 'entity_versions' table so that this function
    // can read out the version.
    let statement_str = format!(
        "SELECT name FROM sqlite_schema WHERE type='table' AND name='{}'",
        DaoManagerT::TABLE_NAME
    );
    let mut statement = dao_manager.get_connection().prepare(statement_str.as_str())
        .expect(format!("Error on preparing statement '{}'", statement_str).as_str());

    let rows = statement.query([])
        .expect(format!("Error on querying statement '{}'", statement_str).as_str());

    let count_rows= rows.count()?;
    let ret_val = if count_rows == 0 {0} else {DaoManagerT::DB_SCHEMA_VERSION} as DbSchemaVersionType;
    Ok(ret_val)
}

pub fn update_db_schema_to_current_version<DaoManagerT: DaoManager>(dao_manager: &DaoManagerT) -> Result<()> {
    let schema_version_in_db = get_schema_version_in_database(dao_manager)?;
    if schema_version_in_db < DaoManagerT::DB_SCHEMA_VERSION {
        dao_manager.init_db_schema()
            .expect(format!("Error on initializing the database for item {} resp. table {}",
                            DaoManagerT::ITEM_TYPE_NAME,
                            DaoManagerT::TABLE_NAME
            ).as_str());
    }
    Ok(())
}

#[derive(Clone)]
pub struct DaoDataStore<DaoManagerT: DaoManager + Clone> {
    _connection: Rc<Connection>,
    items: DaoManagerT,
    _file_path_and_name: String
}

impl<DaoManagerT: DaoManager + Clone> DaoDataStore<DaoManagerT> {

    pub fn open_or_create_db(file_path_and_name: &str) -> Result<Connection>{
        let connection = Connection::open(file_path_and_name)
            .expect(format!("Error on open/create SQlite database file '{}'", file_path_and_name).as_str());
        Ok(connection)
    }

    pub fn new_from_db_file(file_path_and_name: &str) -> Self {
        let connection: Rc<Connection> = Rc::new(Self::open_or_create_db(file_path_and_name).unwrap());
        Self::new_from_connection(connection, Some(String::from(file_path_and_name)))
    }

    pub fn new_from_connection(connection: Rc<Connection>, file_path_and_name: Option<String>) -> Self {
        let items = DaoManagerT::new_from_connection(connection.clone());
        items.update_db_schema_to_current_version()
            .expect(format!("Error on updating database schema for {}.{}",
                            DaoManagerT::DAO_MANAGER_NAME, DaoManagerT::ITEM_TYPE_NAME).as_str());

        DaoDataStore {
            _connection: connection,
            items,
            _file_path_and_name: if let Some(file_name) = file_path_and_name {file_name} else {"No file path and name given".to_string()},
        }
    }

    pub fn search_item(&self, key_starts_with: &str) -> Result<(DaoManagerT::ItemType, DaoManagerT::SerializationCallbackType)>{
        let item = self.items.search_item(key_starts_with)?;
        let serialization_callback = self.items.get_serialization_callback(&item);
        Ok((item, serialization_callback))
    }

    pub fn get_item(&self, key: &DaoManagerT::PrimaryKeyType) -> Result<(DaoManagerT::ItemType, DaoManagerT::SerializationCallbackType)>{
        let item = self.items.get_item_from_db(key)?;
        let serialization_callback = self.items.get_serialization_callback(&item);
        Ok((item, serialization_callback))
    }

    pub fn get_serialization_callback(&self, item: &DaoManagerT::ItemType) -> DaoManagerT::SerializationCallbackType {
        self.items.get_serialization_callback(item)
    }

    pub fn write_item_to_db(&self, item: &DaoManagerT::ItemType) -> Result<DaoManagerT::PrimaryKeyType> {
        self.items.write_item_to_db(item)
    }

    pub fn update_item_in_db(&self, item: &DaoManagerT::ItemType) -> Result<usize> {
        self.items.update_item_in_db(item)
    }

    pub fn delete_item_in_db(&self, key: &DaoManagerT::PrimaryKeyType) -> Result<()> {
        self.items.delete_item_in_db(key)
    }

}

// These tests need to be started as follows:
//      > cargo test --package streams-tools --features dao --lib dao_helpers::tests
//
#[cfg(test)]
mod tests {
    use rusqlite::params;
    use serde_rusqlite::to_params_named;
    use super::*;
    use serde::{
        Serialize,
        Deserialize,
    };
    use crate::helpers::SerializationCallbackRefToClosureString;

    #[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
    pub struct TestItem {
        pub id: String,
        pub some_data: Vec<u8>,
    }

    #[derive(Clone)]
    pub struct TestItemDaoManager {
        connection: Rc<Connection>,
    }

    impl<'a> DaoManager for TestItemDaoManager {
        type ItemType = TestItem;
        type PrimaryKeyType = String;
        type SerializationCallbackType = SerializationCallbackRefToClosureString;

        const ITEM_TYPE_NAME: &'static str = "TestItem";
        const DAO_MANAGER_NAME: &'static str = "TestItemDaoManager";
        const TABLE_NAME: &'static str = "test_item";
        const PRIMARY_KEY_COLUMN_NAME: &'static str = "id";
        const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

        fn new_from_connection(connection: Rc<Connection>) -> Self {
            TestItemDaoManager {
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
            self.connection.execute(format!("CREATE TABLE IF NOT EXISTS {} (\
                {} TEXT NOT NULL PRIMARY KEY,\
                some_data BLOB NOT NULL\
            )
            ", Self::TABLE_NAME, Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
                .expect("Error on executing 'CREATE TABLE' for TestItem");
            Ok(())
        }

        fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<TestItem> {
            get_item_from_db(self, key, None)
        }

        fn search_item(&self, id_starts_with: &str) -> Result<Self::ItemType> {
            get_item_from_db(self, &id_starts_with.to_string(), Some(true))
        }

        fn write_item_to_db(&self, item: &Self::ItemType) -> Result<Self::PrimaryKeyType> {
            let _rows = self.connection.execute(
                format!(
                    "INSERT OR REPLACE INTO {} ({}, some_data) VALUES (:id, :some_data)",
                    Self::TABLE_NAME,
                    Self::PRIMARY_KEY_COLUMN_NAME
                ).as_str(),
                to_params_named(item).unwrap().to_slice().as_slice()
            ).unwrap();
            Ok(item.id.clone())
        }

        fn update_item_in_db(&self, item: &Self::ItemType) -> Result<usize> {
            let rows = self.connection.execute(
                format!(
                    "UPDATE {} SET some_data = :some_data WHERE {} = :id",
                    Self::TABLE_NAME,
                    Self::PRIMARY_KEY_COLUMN_NAME
                ).as_str(),
                to_params_named(item).unwrap().to_slice().as_slice()
            ).unwrap();
            Ok(rows)
        }

        fn delete_item_in_db(&self, key: &Self::PrimaryKeyType) -> Result<()> {
            let _rows = self.connection.execute(
                format!(
                    "DELETE FROM {} WHERE {} = '{}'",
                    Self::TABLE_NAME,
                    Self::PRIMARY_KEY_COLUMN_NAME,
                    key
                ).as_str(),
                params![]
            ).unwrap();
            Ok(())
        }

        fn get_serialization_callback(&self, _item: &TestItem) -> Self::SerializationCallbackType {
            let this = self.clone();
            Box::new( move |id: String, some_data: Vec<u8>| -> Result<usize> {
                let ret_val = some_data.len();
                let new_test_item = TestItem {id, some_data};
                this.write_item_to_db(&new_test_item)?;
                Ok(ret_val)
            })
        }
    }

    pub type TestItemDataStore = DaoDataStore<TestItemDaoManager>;

    #[test]
    fn test_item_dao_manager() {
        let test_item_dao_manager = TestItemDaoManager::new_from_connection(Rc::new(Connection::open_in_memory().unwrap()));
        test_item_dao_manager.init_db_schema().unwrap();

        let test_item = TestItem { id: "test".to_string(), some_data: vec![1, 2, 3, 4] };
        let key = test_item_dao_manager.write_item_to_db(&test_item).unwrap();
        assert_eq!(key, test_item.id);

        let test_item_from_db = test_item_dao_manager.get_item_from_db(&"test".to_string()).unwrap();
        assert_eq!(test_item_from_db, test_item);

        let test_item_from_db = test_item_dao_manager.search_item("test").unwrap();
        assert_eq!(test_item_from_db, test_item);

        test_item_dao_manager.delete_item_in_db(&test_item.id).unwrap();
        match test_item_dao_manager.get_item_from_db(&test_item.id) {
            Ok(item) => {
                assert_eq!(item.id.as_str(), "Should no more exist in db")
            }
            Err(_) => {
                // Everything is fine
            }
        }
    }

    fn create_data_store_with_item_0() -> (TestItemDaoManager, DaoDataStore<TestItemDaoManager>, TestItem, SerializationCallbackRefToClosureString) {
        let test_item_dao_manager = TestItemDaoManager::new_from_connection(
            Rc::new(Connection::open_in_memory().unwrap())
        );
        test_item_dao_manager.init_db_schema().unwrap();

        let test_item_0 = TestItem { id: "item-0".to_string(), some_data: vec![1, 2, 3, 4] };
        let key = test_item_dao_manager.write_item_to_db(&test_item_0).unwrap();
        assert_eq!(key, test_item_0.id);

        let test_item_data_store = TestItemDataStore::new_from_connection(
            test_item_dao_manager.connection.clone(),
            None
        );
        let (test_item_from_db_0, serialization_callback) = test_item_data_store.get_item(&"item-0".to_string()).unwrap();
        assert_eq!(test_item_from_db_0, test_item_0);
        (test_item_dao_manager, test_item_data_store, test_item_0, serialization_callback)
    }

    #[test]
    fn test_dao_data_store() {
        let (test_item_dao_manager,
            test_item_data_store,
            _test_item_0,
            _serialization_callback,
        ) = create_data_store_with_item_0();

        let test_item_1 = TestItem { id: "item-1".to_string(), some_data: vec![5, 6, 7, 8] };
        let key_1 = test_item_dao_manager.write_item_to_db(&test_item_1).unwrap();
        assert_eq!(key_1, test_item_1.id);
        let test_item_from_db_1 = test_item_data_store.get_item(&"item-1".to_string()).unwrap();
        assert_eq!(test_item_from_db_1.0, test_item_1);

        let test_item_plus = TestItem { id: "item+x".to_string(), some_data: vec![9, 8, 7, 6] };
        let key_plus = test_item_dao_manager.write_item_to_db(&test_item_plus).unwrap();
        assert_eq!(key_plus, test_item_plus.id);

        let test_item_from_db_plus = test_item_data_store.get_item(&"item+x".to_string()).unwrap();
        assert_eq!(test_item_from_db_plus.0, test_item_plus);

        let first_item_from_db_plus = test_item_data_store.search_item("item+").unwrap();
        assert_eq!(first_item_from_db_plus.0, test_item_plus);

        test_item_data_store.delete_item_in_db(&test_item_1.id).unwrap();
        match test_item_data_store.get_item(&test_item_1.id) {
            Ok(item_and_cb) => {
                assert_eq!(item_and_cb.0.id.as_str(), "Should no more exist in db")
            }
            Err(_) => {
                // Everything is fine
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_dao_data_store_search_item_fail() {
        let (test_item_dao_manager,
            test_item_data_store,
            _test_item_0,
            _serialization_callback,
        ) = create_data_store_with_item_0();

        let test_item_1 = TestItem { id: "item-1".to_string(), some_data: vec![5, 6, 7, 8] };
        let key_1 = test_item_dao_manager.write_item_to_db(&test_item_1).unwrap();
        assert_eq!(key_1, test_item_1.id);

        let test_item_from_db_1 = test_item_data_store.get_item(&"item-1".to_string()).unwrap();
        assert_eq!(test_item_from_db_1.0, test_item_1);

        // Should panic because there are two items that start with "item-"
        let _first_item_from_db_hyphen = test_item_data_store.search_item("item-").unwrap();
    }

    #[test]
    fn test_dao_data_store_serialization_callback() {
        let (test_item_dao_manager,
            _test_item_data_store,
            mut test_item_0,
            serialization_callback,
        ) = create_data_store_with_item_0();

        // test_item_0.some_data is originally vec![1, 2, 3, 4]
        // adding some more data here
        test_item_0.some_data.push(5);
        test_item_0.some_data.push(6);
        test_item_0.some_data.push(7);

        serialization_callback(test_item_0.id.clone(), test_item_0.some_data.clone()).unwrap();

        let test_item_from_db_0 = test_item_dao_manager.get_item_from_db(&test_item_0.id).unwrap();
        assert_eq!(test_item_from_db_0, test_item_0);
    }

    #[test]
    fn test_dao_data_store_update_item() {
        let (test_item_dao_manager,
            test_item_data_store,
            mut test_item_0,
            _serialization_callback,
        ) = create_data_store_with_item_0();

        // test_item_0.some_data is originally vec![1, 2, 3, 4]
        // adding some more data here
        test_item_0.some_data.push(5);
        test_item_0.some_data.push(6);
        test_item_0.some_data.push(7);

        test_item_data_store.update_item_in_db(&test_item_0).unwrap();

        let test_item_from_db_0 = test_item_dao_manager.get_item_from_db(&test_item_0.id).unwrap();
        assert_eq!(test_item_from_db_0, test_item_0);
    }
}