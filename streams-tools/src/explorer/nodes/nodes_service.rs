use crate::{
    dao_helpers::{
        DaoManager,
        Limit,
        Condition,
        Conditions,
        MatchType,
    },
    user_manager::dao::user::{
        User,
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

use super::{
    NodeConditions,
    Node,
};

fn get_dao_conditions(dto_cond: NodeConditions) -> Vec<Condition> {
    let mut ret_val = Vec::<Condition>::new();
    let mut conditions = Conditions(&mut ret_val);
    conditions.add(dto_cond.channel_id_start, "streams_channel_id", MatchType::StartsWith);
    conditions.add(dto_cond.external_id, "external_id", MatchType::ExactMatch);
    conditions.add(dto_cond.name_start, "name", MatchType::StartsWith);
    ret_val
}

impl From<&User> for Node {
    fn from(value: &User) -> Self {
        Node {
            channel_id: value.streams_channel_id.clone(),
            name: value.name.clone(),
            external_id: value.external_id.clone()
        }
    }
}
impl From<User> for Node {
    fn from(value: User) -> Self { (&value).into() }
}

pub(crate) fn index(conditions: NodeConditions, user_store: &UserDataStore, paging_opt: Option<PagingOptions>) -> Result<(Vec<Node>, usize)> {
    let db_limit_offset = paging_opt.map(|paging_opt| Limit::from(paging_opt));
    let dao_conditions = get_dao_conditions(conditions);
    let (users, items_cnt_total) = user_store.filter(dao_conditions, db_limit_offset)?;
    let nodes = users.iter().map(|user| user.into() ).collect();

    Ok((nodes, items_cnt_total))
}

pub(crate) fn get(channel_id: &<UserDaoManager as DaoManager>::PrimaryKeyType, user_store: &UserDataStore) -> Result<Node> {
    let user = match user_store.get_item_read_only(channel_id) {
        Ok(user) => user,
        Err(_) => {
            return Err(AppError::ChannelDoesNotExist(channel_id.to_string()))
        }
    };
    Ok(user.into())
}


pub(crate) fn put(channel_id: &<UserDaoManager as DaoManager>::PrimaryKeyType, user_store: &UserDataStore, node: Node) -> Result<Node> {
    let user = match user_store.get_item(channel_id) {
        Ok((mut user, _)) => {
            user.name = node.name;
            user.external_id = node.external_id;
            user_store.write_item_to_db(&user)?;
            user
        },
        Err(_) => {
            return Err(AppError::ChannelDoesNotExist(channel_id.to_string()))
        }
    };
    Ok(user.into())
}