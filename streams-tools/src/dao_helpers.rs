use std::fmt;

use rusqlite::{
    Connection,
    Rows,
    Statement
};

use serde_rusqlite::from_rows_ref;
use serde::de::{
    DeserializeOwned,
};

use anyhow::{
    Result,
    bail,
};

use fallible_streaming_iterator::FallibleStreamingIterator;

pub type DbSchemaVersionType = i32;

pub struct Limit {
    pub limit: usize,
    pub offset: usize,
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LIMIT {} OFFSET {}", self.limit, self.offset)
    }
}

const MAX_NUMBER_OF_ROWS_TO_FETCH: usize = 1000;

pub trait DaoManager {

    type ItemType: DeserializeOwned;
    type PrimaryKeyType: fmt::Display + ToString;
    type SerializationCallbackType;
    type OptionsType;

    const ITEM_TYPE_NAME: &'static str;
    const DAO_MANAGER_NAME: &'static str;
    const PRIMARY_KEY_COLUMN_NAME: &'static str;
    const DB_SCHEMA_VERSION: DbSchemaVersionType;

    fn new(options: Self::OptionsType) -> Self;

    fn get_table_name(&self) -> String;

    fn get_connection(&self) -> &Connection;

    fn update_db_schema_to_current_version(&self) -> Result<()>;

    fn init_db_schema(&self) -> Result<()>;

    fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<Self::ItemType>;

    fn search_item(&self, key_starts_with: &str) -> Result<Self::ItemType>;

    fn find_all(&self, key_starts_with: &str, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)>;

    fn filter(&self, conditions: Vec<Condition>, limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)>;

    fn write_item_to_db(&self, item: &Self::ItemType) -> Result<Self::PrimaryKeyType>;

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

#[derive(Clone, PartialEq)]
pub enum MatchType {
    ExactMatch,
    StartsWith,
    ListEverything,
}

fn get_rows_from_statement<'a>(statement: &'a mut Statement, statement_str: &String, item_type_name: &str) -> Rows<'a> {
    statement.query([])
        .expect(format!("Error on querying prepared 'SELECT ... FROM' statement for {}. Statement: {}",
                        item_type_name,
                        statement_str).as_str())
}

fn get_value_from_statement<'a, ValueTypeT>(statement: &'a mut Statement, statement_str: &String, item_type_name: &str) -> Vec<ValueTypeT>
where
    ValueTypeT: DeserializeOwned
{
    let mut rows = get_rows_from_statement(statement, &statement_str, item_type_name);
    let mut res = from_rows_ref::<ValueTypeT>(&mut rows);
    let mut ret_val = Vec::<ValueTypeT>::new();
    while let Some(item) = res.next() {
        let to_push = item
            .expect(format!("Error on unwrapping next {} from_rows_ref", item_type_name).as_str());
        ret_val.push(to_push);
    }
    ret_val
}

pub fn find_all_items_in_db<'a, DaoManagerT: DaoManager>(
    dao_manager: &DaoManagerT,
    starts_with: &DaoManagerT::PrimaryKeyType,
    limit: Option<Limit>
) -> Result<(Vec<DaoManagerT::ItemType>, usize)>
{
    let match_type = if starts_with.to_string().len() > 0 {
        MatchType::StartsWith
    } else {
        MatchType::ListEverything
    };
    let conditions = vec![Condition{
        column_name: DaoManagerT::PRIMARY_KEY_COLUMN_NAME.to_string(),
        column_value: starts_with.to_string(),
        match_type
    }];
    filter_items(dao_manager, &conditions, limit)
}

pub fn filter_items<'a, DaoManagerT: DaoManager>(
    dao_manager: &DaoManagerT,
    conditions: &Vec<Condition>,
    limit: Option<Limit>
) -> Result<(Vec<DaoManagerT::ItemType>, usize)>
{
    let limit_offset =  limit.or(Some(
        Limit{
            limit: MAX_NUMBER_OF_ROWS_TO_FETCH,
            offset: 0
        }
    ));
    let (mut statement, statement_str) = build_query_statement(dao_manager, conditions, limit_offset)?;
    let ret_val = get_value_from_statement::<DaoManagerT::ItemType>(&mut statement, &statement_str, DaoManagerT::ITEM_TYPE_NAME);

    let (mut cnt_statement, cnt_statement_str) = build_select_count_statement(dao_manager, conditions)?;
    let counts = get_value_from_statement::<usize>(&mut cnt_statement, &cnt_statement_str, "count");

    Ok((ret_val, counts[0]))
}

pub fn get_item_from_db<'a, DaoManagerT: DaoManager>(
    dao_manager: &DaoManagerT,
    primary_key: &DaoManagerT::PrimaryKeyType,
    match_type: MatchType
) -> Result<DaoManagerT::ItemType>
{
    let conditions = vec![Condition{
        column_name: DaoManagerT::PRIMARY_KEY_COLUMN_NAME.to_string(),
        column_value: primary_key.to_string(),
        match_type
    }];
    let (mut statement, statement_str) = build_query_statement(dao_manager, &conditions, None)?;
    let rows = get_rows_from_statement(&mut statement, &statement_str, DaoManagerT::ITEM_TYPE_NAME);
    get_item_from_single_row_rowset(dao_manager, primary_key, rows, statement_str)
}

fn get_item_from_single_row_rowset<'a, DaoManagerT: DaoManager>(
    dao_manager: &DaoManagerT,
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
                dao_manager.get_table_name(),
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
            dao_manager.get_table_name(),
            DaoManagerT::PRIMARY_KEY_COLUMN_NAME,
            key,
            statement_str,
        )
    }
}


#[derive(Clone)]
pub struct Condition {
    column_name: String,
    column_value: String,
    match_type: MatchType
}

pub struct Conditions<'a>(pub &'a mut Vec<Condition>);

impl<'a> Conditions<'a> {
    pub fn add(&mut self, opt_value: Option<String>, column_name: &str, match_type: MatchType) {
        if let Some(column_value) = opt_value {
            (*self.0).push(
                Condition{
                    column_name: column_name.to_string(),
                    column_value,
                    match_type
                }
            );
        }
    }

    pub fn has_any_where_part(&self) -> bool {
        (*self.0).iter().filter(|cond| cond.match_type != MatchType::ListEverything ).count() > 0
    }
}

#[derive(Clone)]
struct WherePart {
    column_name: String,
    column_value: String,
    operator: String,
    wildcard: String,
}

impl From<Condition> for WherePart {
    fn from(value: Condition) -> Self {
        let mut operator = "=";
        let mut wildcard = "";
        if value.match_type == MatchType::StartsWith {
            operator = "LIKE";
            wildcard = "%";
        };
        WherePart{
            column_name: value.column_name.to_string(),
            column_value: value.column_value.to_string(),
            operator: operator.to_string(),
            wildcard: wildcard.to_string(),
        }
    }
}

#[derive(Clone)]
struct StatementParts {
    where_parts: Vec<WherePart>,
    needs_where_part: bool,
    limit_offset: String,
}

impl StatementParts {
    pub fn new<'a, DaoManagerT: DaoManager>(limit: Option<Limit>, mut conditions: Vec<Condition>) -> StatementParts {
        let limit_offset = limit.map_or("".to_string(), |lim_offset| lim_offset.to_string());
        let mut ret_val = StatementParts {
            where_parts: Vec::new(),
            needs_where_part: Conditions(&mut conditions).has_any_where_part(),
            limit_offset,
        };
        ret_val.add_where_parts(conditions);
        ret_val
    }
    
    pub fn add_where_parts(&mut self, conditions: Vec<Condition>) {
        for cond in conditions {
            if cond.match_type != MatchType::ListEverything {
                self.where_parts.push(cond.into())
            }
        }
    }
}

fn build_query_statement<'a, DaoManagerT: DaoManager>(
    dao_manager: &'a DaoManagerT,
    conditions: &Vec<Condition>,
    limit: Option<Limit>,
) -> Result<(Statement<'a>, String)>
{
    let parts = StatementParts::new::<DaoManagerT>(limit, conditions.clone());
    let where_condition = get_where_condition::<DaoManagerT>(parts.clone());
    let statement_str = format!("SELECT * FROM {t_name} {where} {lim_ofs}",
                                t_name = dao_manager.get_table_name(),
                                where = where_condition,
                                lim_ofs = parts.limit_offset,
    );
    let statement = dao_manager.get_connection().prepare(statement_str.as_str())
        .expect(format!("Error on preparing 'SELECT * FROM' for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    Ok((statement, statement_str))
}

fn get_where_condition<DaoManagerT: DaoManager>(parts: StatementParts) -> String {
    if parts.needs_where_part {
        let mut ret_val = "WHERE ".to_string();
        for (idx, where_part) in parts.where_parts.iter().enumerate() {
            if idx > 0 {
                ret_val += " AND ";
            }
            ret_val += format!("{col_name} {op} '{col_val}{wldcrd}'",
                    col_name = where_part.column_name,
                    col_val = where_part.column_value,
                    op = where_part.operator,
                    wldcrd = where_part.wildcard,
            ).as_str();
        }
        ret_val
    } else {
        "".to_string()
    }
}

fn build_select_count_statement<'a, DaoManagerT: DaoManager>(
    dao_manager: &'a DaoManagerT,
    conditions: &Vec<Condition>,
) -> Result<(Statement<'a>, String)>
{
    let parts = StatementParts::new::<DaoManagerT>(None, conditions.clone());
    let where_condition = get_where_condition::<DaoManagerT>(parts);
    let statement_str = format!("SELECT COUNT(*) FROM {t_name} {where}",
                                t_name = dao_manager.get_table_name(),
                                where = where_condition,
    );
    let statement = dao_manager.get_connection().prepare(statement_str.as_str())
        .expect(format!("Error on preparing 'SELECT COUNT(*) FROM' for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    Ok((statement, statement_str))
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
        dao_manager.get_table_name()
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
                            dao_manager.get_table_name()
            ).as_str());
    }
    Ok(())
}

#[derive(Clone)]
pub struct DaoDataStore<DaoManagerT: DaoManager + Clone> {
    items: DaoManagerT,
}

impl<DaoManagerT: DaoManager + Clone> DaoDataStore<DaoManagerT> {

    pub fn new(options: DaoManagerT::OptionsType) -> Self {
        let items = DaoManagerT::new(options);
        items.update_db_schema_to_current_version()
            .expect(format!("Error on updating database schema for {}.{}",
                            DaoManagerT::DAO_MANAGER_NAME, DaoManagerT::ITEM_TYPE_NAME).as_str());

        DaoDataStore {
            items,
        }
    }

    pub fn search_item(&self, key_starts_with: &str) -> Result<(DaoManagerT::ItemType, DaoManagerT::SerializationCallbackType)>{
        let item = self.items.search_item(key_starts_with)?;
        let serialization_callback = self.items.get_serialization_callback(&item);
        Ok((item, serialization_callback))
    }

    pub fn find_all(&self, key_starts_with: &str, limit: Option<Limit>) -> Result<(Vec<DaoManagerT::ItemType>, usize)> {
        self.items.find_all(key_starts_with, limit)
    }

    pub fn filter(&self, conditions: Vec<Condition>, limit: Option<Limit>) -> Result<(Vec<DaoManagerT::ItemType>, usize)> {
        self.items.filter(conditions, limit)
    }

    pub fn get_item(&self, key: &DaoManagerT::PrimaryKeyType) -> Result<(DaoManagerT::ItemType, DaoManagerT::SerializationCallbackType)>{
        let item = self.items.get_item_from_db(key)?;
        let serialization_callback = self.items.get_serialization_callback(&item);
        Ok((item, serialization_callback))
    }

    pub fn get_item_read_only(&self, key: &DaoManagerT::PrimaryKeyType) -> Result<DaoManagerT::ItemType> {
        self.items.get_item_from_db(key)
    }

    pub fn get_serialization_callback(&self, item: &DaoManagerT::ItemType) -> DaoManagerT::SerializationCallbackType {
        self.items.get_serialization_callback(item)
    }

    pub fn write_item_to_db(&self, item: &DaoManagerT::ItemType) -> Result<DaoManagerT::PrimaryKeyType> {
        self.items.write_item_to_db(item)
    }

    pub fn delete_item_in_db(&self, key: &DaoManagerT::PrimaryKeyType) -> Result<()> {
        self.items.delete_item_in_db(key)
    }
}

#[derive(Clone)]
pub struct DbFileBasedDaoManagerOptions {
    pub file_path_and_name: String,
}

pub trait DbFileBasedDaoManagerOpt: Clone {
    fn file_path_and_name(&self) -> String;

    fn get_new_connection(&self) -> Connection {
        Connection::open(self.file_path_and_name())
            .expect(format!("Error on open/create SQlite database file '{}'", self.file_path_and_name()).as_str())
    }
}

#[cfg(not(test))]
impl DbFileBasedDaoManagerOpt for DbFileBasedDaoManagerOptions {
    fn file_path_and_name(&self) -> String {
        self.file_path_and_name.clone()
    }
}

#[cfg(test)]
impl DbFileBasedDaoManagerOpt for DbFileBasedDaoManagerOptions {
    fn file_path_and_name(&self) -> String { "NOT USED".to_string() }

    fn get_new_connection(&self) -> Connection {
        Connection::open_in_memory().unwrap()
    }
}

// These tests need to be started as follows:
//      > cargo test --package streams-tools --features dao --lib dao_helpers::tests
//
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
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
        connection: Arc<Mutex<Connection>>,
    }

    #[derive(Clone)]
    pub struct TestItemDaoManagerOptions{
        pub connection: Arc<Mutex<Connection>>
    }

    impl<'a> DaoManager for TestItemDaoManager {
        type ItemType = TestItem;
        type PrimaryKeyType = String;
        type SerializationCallbackType = SerializationCallbackRefToClosureString;
        type OptionsType = TestItemDaoManagerOptions;

        const ITEM_TYPE_NAME: &'static str = "TestItem";
        const DAO_MANAGER_NAME: &'static str = "TestItemDaoManager";
        const PRIMARY_KEY_COLUMN_NAME: &'static str = "id";
        const DB_SCHEMA_VERSION: DbSchemaVersionType = 1;

        fn new(options: TestItemDaoManagerOptions) -> Self {
            TestItemDaoManager {
                connection: options.connection
            }
        }

        fn get_connection(&self) -> &Connection {
            unimplemented!()
        }

        fn get_table_name(&self) -> String { "test_item".to_string() }

        fn update_db_schema_to_current_version(&self) -> Result<()> {
            update_db_schema_to_current_version(self)
        }

        fn init_db_schema(&self) -> Result<()> {
            let connection = self.connection.lock().unwrap();
            connection.execute(format!("CREATE TABLE IF NOT EXISTS {} (\
                {} TEXT NOT NULL PRIMARY KEY,\
                some_data BLOB NOT NULL\
            )
            ", self.get_table_name(), Self::PRIMARY_KEY_COLUMN_NAME).as_str(), [])
                .expect("Error on executing 'CREATE TABLE' for TestItem");
            Ok(())
        }

        fn get_item_from_db(&self, key: &Self::PrimaryKeyType) -> Result<TestItem> {
            get_item_from_db(self, key, MatchType::ExactMatch)
        }

        fn search_item(&self, id_starts_with: &str) -> Result<Self::ItemType> {
            get_item_from_db(self, &id_starts_with.to_string(),  MatchType::StartsWith)
        }

        fn find_all(&self, _key_starts_with: &str, _limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
            unimplemented!()
        }

        fn write_item_to_db(&self, item: &Self::ItemType) -> Result<Self::PrimaryKeyType> {
            let connection = self.connection.lock().unwrap();
            let _rows = connection.execute(
                format!(
                    "INSERT OR REPLACE INTO {} ({}, some_data) VALUES (:id, :some_data)",
                    self.get_table_name(),
                    Self::PRIMARY_KEY_COLUMN_NAME
                ).as_str(),
                to_params_named(item).unwrap().to_slice().as_slice()
            ).unwrap();
            Ok(item.id.clone())
        }

        fn delete_item_in_db(&self, key: &Self::PrimaryKeyType) -> Result<()> {
            let connection = self.connection.lock().unwrap();
            let _rows = connection.execute(
                format!(
                    "DELETE FROM {} WHERE {} = '{}'",
                    self.get_table_name(),
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

        fn filter(&self, _conditions: Vec<Condition>, _limit: Option<Limit>) -> Result<(Vec<Self::ItemType>, usize)> {
            todo!()
        }
    }

    pub type TestItemDataStore = DaoDataStore<TestItemDaoManager>;

    #[test]
    fn test_item_dao_manager() {
        let options = TestItemDaoManagerOptions {
            connection: Arc::new(Mutex::new(
                Connection::open_in_memory().unwrap()
            ))
        };
        let test_item_dao_manager = TestItemDaoManager::new(options);
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
        let options = TestItemDaoManagerOptions {
            connection: Arc::new(Mutex::new(
                Connection::open_in_memory().unwrap()
            ))
        };
        let test_item_dao_manager = TestItemDaoManager::new(options.clone());
        test_item_dao_manager.init_db_schema().unwrap();

        let test_item_0 = TestItem { id: "item-0".to_string(), some_data: vec![1, 2, 3, 4] };
        let key = test_item_dao_manager.write_item_to_db(&test_item_0).unwrap();
        assert_eq!(key, test_item_0.id);

        let test_item_data_store = TestItemDataStore::new(options);
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
}