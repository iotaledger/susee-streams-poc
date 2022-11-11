use rusqlite::{
    Connection,
    Rows,
};
use serde_rusqlite::from_rows_ref;

use anyhow::{
    Result,
    bail,
};
use fallible_streaming_iterator::FallibleStreamingIterator;
use serde::de::DeserializeOwned;

pub type DbSchemaVersionType = i32;

pub trait DaoManager {

    type ItemType;
    const ITEM_TYPE_NAME: &'static str;
    const TABLE_NAME: &'static str;
    const PRIMARY_KEY_COLUMN_NAME: &'static str;
    const DB_SCHEMA_VERSION: DbSchemaVersionType;

    fn get_connection(&self) -> &Connection;

    fn update_db_schema_to_current_version(&self) -> Result<()>;

    fn init_db_schema(&self) -> Result<()>;

    fn get_item_from_db(&self, key: &str) -> Result<Self::ItemType>;

    fn write_item_to_db(&self, item: Self::ItemType) -> Result<usize>;

    fn update_item_in_db(&self, item: Self::ItemType) -> Result<usize>;
}

pub fn get_item_from_db<'de, DaoManagerT>(dao_manager: &DaoManagerT, key: &str, starts_with: Option<bool>) -> Result<DaoManagerT::ItemType>
    where
        DaoManagerT: DaoManager,
        DaoManagerT::ItemType: DeserializeOwned,
{
    let statement_str = build_query_statement(dao_manager, key, starts_with)?;
    let mut statement = dao_manager.get_connection().prepare(statement_str.as_str())
        .expect(format!("Error on preparing 'SELECT * FROM' for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    let rows = statement.query([])
        .expect(format!("Error on querying prepared 'SELECT * FROM' statement for {}. Statement: {}",
                        DaoManagerT::ITEM_TYPE_NAME,
                        statement_str).as_str());
    get_item_from_single_row_rowset(dao_manager, key, rows, statement_str)
}

fn get_item_from_single_row_rowset<'de, DaoManagerT>(_dao_manager: &DaoManagerT, key: &str, mut rows: Rows, statement_str: String) -> Result<DaoManagerT::ItemType>
where
    DaoManagerT: DaoManager,
    DaoManagerT::ItemType: DeserializeOwned,
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

fn build_query_statement<DaoManagerT: DaoManager>(_dao_manager: &DaoManagerT, key: &str, starts_with: Option<bool>) -> Result<String> {
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
                            key,
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