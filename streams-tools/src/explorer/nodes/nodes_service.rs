use crate::{
    dao_helpers::{
        DaoManager,
        Limit,
    },
    user_manager::dao::user::{
        UserDataStore,
        UserDaoManager,
    },
    explorer::{
        error::{
            Result,
            AppError,
        },
        shared::PagingOptions,
    },
};

use super::nodes_dto::Node;

pub(crate) fn index(channel_id_start: &str, user_store: &UserDataStore, paging_opt: Option<PagingOptions>) -> Result<(Vec<Node>, usize)> {
    let db_limit_offset = paging_opt.map(|paging_opt| Limit::from(paging_opt));
    let (users, items_cnt_total) = user_store.find_all(channel_id_start, db_limit_offset)?;
    let nodes = users.iter().map(|user| Node { channel_id: user.streams_channel_id.clone() }).collect();

    Ok((nodes, items_cnt_total))
}

pub(crate) fn get(channel_id: &<UserDaoManager as DaoManager>::PrimaryKeyType, user_store: &UserDataStore) -> Result<Node> {
    let user = match user_store.get_item(channel_id) {
        Ok((user, _)) => user,
        Err(_) => {
            return Err(AppError::ChannelDoesNotExist(channel_id.to_string()))
        }
    };
    Ok(Node { channel_id: user.streams_channel_id.clone() })
}