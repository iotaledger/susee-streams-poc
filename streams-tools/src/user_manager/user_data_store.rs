use rusqlite::Connection;
use crate::{
    dao_helpers::DaoManager,
    dao::user::{
        User,
        UserDaoManager
    },
};
use anyhow::Result;
use crate::helpers::SerializationCallbackRefToClosure;
use std::rc::Rc;

#[derive(Clone)]
pub struct UserDataStore {
    _connection: Rc<Connection>,
    users: UserDaoManager,
    _file_path_and_name: String
}

impl UserDataStore {

    pub fn open_or_create_db(file_path_and_name: &str) -> Result<Connection>{
        let connection = Connection::open(file_path_and_name)
            .expect(format!("Error on open/create SQlite database file '{}'", file_path_and_name).as_str());
        Ok(connection)
    }

    pub fn new_from_db_file(file_path_and_name: &str) -> Self {
        let connection: Rc<Connection> = Rc::new(UserDataStore::open_or_create_db(file_path_and_name).unwrap());
        let users = UserDaoManager::new_from_connection(connection.clone());
        users.update_db_schema_to_current_version()
            .expect("Error on updatin database schema for UserDataStore.users");

        UserDataStore {
            _connection: connection,
            users,
            _file_path_and_name: String::from(file_path_and_name),
        }
    }

    pub fn search_user_state(&mut self, channel_starts_with: &str) -> Result<(User, SerializationCallbackRefToClosure)>{
        let user = self.users.search_user(channel_starts_with)?;
        let serialization_callback = self.get_serialization_callback(user.seed_derivation_phrase.as_str());
        Ok((user, serialization_callback))
    }

    pub fn get_user_state(&mut self, channel_id: &str) -> Result<(User, SerializationCallbackRefToClosure)>{
        let user = self.users.get_user(channel_id)?;
        let serialization_callback = self.get_serialization_callback(user.seed_derivation_phrase.as_str());
        Ok((user, serialization_callback))
    }

    pub fn get_serialization_callback(&self, seed_derivation_phrase: &str) -> SerializationCallbackRefToClosure {
        let this = self.clone();
        let seed_derive_phrase = String::from(seed_derivation_phrase);
        Box::new( move |streams_channel_id: String, user_state: Vec<u8>| -> Result<usize> {
                let mut new_user = User::default();
                new_user.streams_user_state = user_state;
                new_user.streams_channel_id = streams_channel_id;
                new_user.seed_derivation_phrase = seed_derive_phrase.clone();
                this.users.write_item_to_db(new_user)
        })
    }
}