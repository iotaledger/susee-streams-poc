use std::{
    time::SystemTime,
    hash::{
        Hash,
        Hasher
    },
    collections::hash_map::DefaultHasher
};

use anyhow::{
    Result,
    bail,
};

use crate::{
    channel_manager::{
        ChannelManagerOptions,
    },
    ChannelManager,
    helpers::{
        SerializationCallbackRefToClosureString
    },
    PlainTextWallet,
    UserDataStore,
    user_manager::dao::User
};

#[derive(Clone)]
pub struct MultiChannelManagerOptions {
    pub iota_node: String,
    pub wallet_filename: String,
    //TODO: Needs to be managed by stronghold
    pub streams_user_serialization_password: String,
}

pub async fn get_initial_channel_manager<'a>(user_store: &UserDataStore, options: &MultiChannelManagerOptions) -> Result<ChannelManager<PlainTextWallet>> {
    let mut new_opt = ChannelManagerOptions::default();
    let wallet = get_wallet(options, None)?;
    let mut initial_user_having_seed_derivation_phrase = User::default();
    initial_user_having_seed_derivation_phrase.seed_derivation_phrase = wallet.seed_derivation_phrase.as_ref().unwrap().clone();
    new_opt.serialize_user_state_callback = Some(
        user_store.get_serialization_callback(&initial_user_having_seed_derivation_phrase)
    );

    Ok( ChannelManager::new(
        options.iota_node.as_str(),
        wallet,
        Some(new_opt)
    ).await)
}

pub async fn get_channel_manager_for_channel_starts_with(channel_starts_with: &str, user_store: &mut UserDataStore, options: &MultiChannelManagerOptions, update_user_on_exit: bool) -> Result<ChannelManager<PlainTextWallet>> {
    if let Ok((user_dao, serialize_user_state_callback)) = user_store.search_item(channel_starts_with) {
        let wallet = get_wallet(options, Some(&user_dao))?;
        let user_state_callback = if update_user_on_exit {
            Some(serialize_user_state_callback)
        } else {
            None
        };
        get_channel_manager_by_user_dao(
            user_dao,
            user_state_callback,
            wallet,
            options.iota_node.as_str(),
        ).await
    } else {
        bail!("Could not find matching Streams channel for ID starting with '{}'", channel_starts_with)
    }
}

pub async fn get_channel_manager_for_channel_id<'a>(channel_id: &str, user_store: &UserDataStore, options: &MultiChannelManagerOptions) -> Result<ChannelManager<PlainTextWallet>> {
    let (user_dao, serialize_user_state_callback) = user_store.get_item(&channel_id.to_string())?;
    let wallet = get_wallet(options, Some(&user_dao))?;
    get_channel_manager_by_user_dao(
        user_dao,
        Some(serialize_user_state_callback),
        wallet,
        options.iota_node.as_str(),
    ).await
}

async fn get_channel_manager_by_user_dao(user_dao: User, serialize_user_state_callback: Option<SerializationCallbackRefToClosureString>, wallet: PlainTextWallet, node: &str) -> Result<ChannelManager<PlainTextWallet>>{
    let mut new_opt = ChannelManagerOptions::default();
    new_opt.user_state = Some(user_dao.streams_user_state.clone());
    new_opt.serialize_user_state_callback = serialize_user_state_callback;
    Ok( ChannelManager::new(
        node,
        wallet,
        Some(new_opt)
    ).await)
}

fn get_wallet(options: &MultiChannelManagerOptions, user_dao: Option<&User>) -> Result<PlainTextWallet>{
    let seed_derivation_phrase = if let Some(user) = user_dao {
        user.seed_derivation_phrase.clone()
    } else {
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("{}", hasher.finish())
    };

    Ok(PlainTextWallet::new(
        options.streams_user_serialization_password.as_str(),
        Some(options.wallet_filename.as_str()),
        Some(seed_derivation_phrase),
    ))
}