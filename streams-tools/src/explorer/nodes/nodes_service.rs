use crate::{
    dao_helpers::DaoManager,
    user_manager::dao::user::{
        UserDataStore,
        UserDaoManager,
    },
    explorer::error::Result,
};

use super::nodes_dto::Node;

pub(crate) fn index(channel_id_start: &str, user_store: &UserDataStore) -> Result<Vec<Node>> {
    let users: Vec<<UserDaoManager as DaoManager>::ItemType> = user_store.find_all(channel_id_start)?;
    let nodes = users.iter().map(|user| Node { channel_id: user.streams_channel_id.clone() }).collect();
    Ok(nodes)
}

pub(crate) fn get(channel_id: &<UserDaoManager as DaoManager>::PrimaryKeyType, user_store: &UserDataStore) -> Result<Node> {
    let (user, _) = user_store.get_item(channel_id)?;
    Ok(Node { channel_id: user.streams_channel_id.clone() })
}