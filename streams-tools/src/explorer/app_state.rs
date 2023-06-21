use std::{
    sync::Arc,
    ops::Deref,
};

use crate::{
    UserDataStore
};

#[derive(Clone)]
pub struct MessagesState {
    pub iota_node_url: String,
    pub wallet_filename: String,
    pub db_file_name: String,
    //TODO: Needs to be managed by stronghold
    pub streams_user_serialization_password: String,
}
unsafe impl Send for MessagesState {}
unsafe impl Sync for MessagesState {}

pub struct AppStateInner {
    pub messages: MessagesState,
    pub user_store: UserDataStore,
}

unsafe impl Send for AppStateInner {}
unsafe impl Sync for AppStateInner {}

pub(crate) struct AppState(Arc<AppStateInner>);

impl Deref for AppState {
    type Target = Arc<AppStateInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl AppState {
    pub fn new(messages_state: MessagesState, user_store: UserDataStore) -> AppState {
        AppState(
            Arc::new(AppStateInner {
                messages: messages_state,
                user_store,
            })
        )
    }
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}