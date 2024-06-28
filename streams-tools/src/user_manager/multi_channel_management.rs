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
    user_manager::dao::{
        User,
        message::MessageDataStoreOptions,
    },
    dao_helpers::{
        Condition,
        Conditions,
        MatchType
    }
};

#[derive(Clone)]
pub struct MultiChannelManagerOptions {
    pub iota_node: String,
    pub wallet_filename: String,
    //TODO: Needs to be managed by stronghold
    pub streams_user_serialization_password: String,
    pub message_data_store_for_msg_caching: Option<MessageDataStoreOptions>,
    pub inx_collector_access_throttle_sleep_time_millisecs: Option<u64>,
}

// To avoid multiple users with conflicting external_ids a name disambiguation mechanism
// similar to conflicting filenames is used:
// * If an external_id is already used, the existing users external_id is extended by an extension consisting
//   of "-(###)" where ### denotes an integer counter that can be interpreted similar to the initialization_cnt used
//   used during the reinitialization workflow.
// * To find out the value of the ### integer counter, the number of existing users with matching external_ids
//   needs to be checked. This is done by searching for external_id values which start with "EXTERNAL_ID_VALUE-("
//   The counter then is the number of existing users.
// * Examples or an external_id "ABCD":
//          // No user with matching external_id exists:
//          // No disambiguation needed:
//          Existing:                []
//          After disambiguation:    []
//          After new user creation: ["ABCD"]
//          // One user with matching external_id exists. The new user receives the pure "ABCD" external_id.
//          // The already existing user dataset is altered to get the external_id "ABCD-(1)":
//          Existing:                [        "ABCD"]
//          After disambiguation:    [        "ABCD-(0)"]
//          After new user creation: ["ABCD", "ABCD-(0)"]
//          // Two users with matching external_id exist. The new user receives the pure "ABCD" external_id
//          // The already existing user dataset with the external_id "ABCD-(0)" stays unchanged.
//          // The already existing user dataset with the external_id "ABCD" is altered to get the external_id "ABCD-(1)":
//          Existing:                [      , "ABCD-(0)", "ABCD"]
//          After disambiguation:    [      , "ABCD-(0)", "ABCD-(1)"]
//          After new user creation: ["ABCD", "ABCD-(0)", "ABCD-(1)"]
async fn manage_existing_users_with_conflicting_external_id(user_store: &UserDataStore, external_id: &str) -> Result<()>{
    let exact_match_conditions = create_conditions_for_external_id_filter(external_id, MatchType::ExactMatch);
    let (mut exact_match_users, _) = user_store.filter(exact_match_conditions, None)?;
    if exact_match_users.len() > 0 {
        let search_string = external_id.to_string() + "-(";
        let already_disambiguated_users_cond = create_conditions_for_external_id_filter(search_string.as_str(), MatchType::StartsWith);
        let (already_disambiguated_users, _) = user_store.filter(already_disambiguated_users_cond, None)?;
        let init_cnt = already_disambiguated_users.len();
        for exact_matching_user in exact_match_users.iter_mut() {
            let old_external_id = exact_matching_user.external_id.clone();
            exact_matching_user.external_id = format!("{}-({})", exact_matching_user.external_id, init_cnt);
            log::info!("[fn manage_existing_users_with_conflicting_external_id()] Replacing external_id of user {}:\nold external_id: {}\nnew external_id: {}",
               exact_matching_user.streams_channel_id,
               old_external_id,
               exact_matching_user.external_id
            );
            user_store.write_item_to_db(exact_matching_user)?;
        }
    }

    Ok(())
}

pub async fn get_initial_channel_manager<'a>(user_store: &UserDataStore, options: &MultiChannelManagerOptions, external_user_id: Option<String>) -> Result<ChannelManager<PlainTextWallet>> {
    let mut new_opt = ChannelManagerOptions::default();
    new_opt.throttle_sleep_time_millisecs = options.inx_collector_access_throttle_sleep_time_millisecs;
    let wallet = get_wallet(options, None)?;
    let mut initial_user_having_seed_derivation_phrase = User::default();
    if let Some(external_id) = external_user_id {
        manage_existing_users_with_conflicting_external_id(user_store,external_id.as_str()).await?;
        initial_user_having_seed_derivation_phrase.external_id = external_id;
    }
    initial_user_having_seed_derivation_phrase.seed_derivation_phrase = wallet.seed_derivation_phrase.as_ref().unwrap().clone();
    new_opt.serialize_streams_client_state_callback = Some(
        user_store.get_serialization_callback(&initial_user_having_seed_derivation_phrase)
    );

    Ok( ChannelManager::new(
        options.iota_node.as_str(),
        wallet,
        Some(new_opt)
    ).await)
}

pub async fn get_channel_manager_for_channel_starts_with(channel_starts_with: &str, user_store: &mut UserDataStore, options: &MultiChannelManagerOptions, update_user_on_exit: bool) -> Result<ChannelManager<PlainTextWallet>> {
    if let Ok((user_dao, serialize_streams_client_state_callback)) = user_store.search_item(channel_starts_with) {
        let wallet = get_wallet(options, Some(&user_dao))?;
        let client_state_callback = if update_user_on_exit {
            Some(serialize_streams_client_state_callback)
        } else {
            None
        };
        get_channel_manager_by_user_dao(
            user_dao,
            wallet,
            options.iota_node.as_str(),
            client_state_callback,
            options.message_data_store_for_msg_caching.clone(),
            options.inx_collector_access_throttle_sleep_time_millisecs,
        ).await
    } else {
        bail!("Could not find matching Streams channel for ID starting with '{}'", channel_starts_with)
    }
}

pub async fn get_channel_manager_for_channel_id<'a>(channel_id: &str, user_store: &UserDataStore, options: &MultiChannelManagerOptions) -> Result<ChannelManager<PlainTextWallet>> {
    let (user_dao, serialize_streams_client_state_callback) =
        user_store.get_item(&channel_id.to_string())?;
    let wallet = get_wallet(options, Some(&user_dao))?;
    get_channel_manager_by_user_dao(
        user_dao,
        wallet,
        options.iota_node.as_str(),
        Some(serialize_streams_client_state_callback),
        options.message_data_store_for_msg_caching.clone(),
        options.inx_collector_access_throttle_sleep_time_millisecs,
    ).await
}

pub async fn get_channel_manager_for_external_id<'a>(external_id: &str, user_store: &UserDataStore, options: &MultiChannelManagerOptions) -> Result<ChannelManager<PlainTextWallet>> {
    let conditions_buffer = create_conditions_for_external_id_filter(external_id, MatchType::ExactMatch);
    if let Some(user) = user_store.get_first_filtered_item(conditions_buffer) {
        get_channel_manager_for_channel_id(user.streams_channel_id.as_str(), user_store, options).await
    } else {
        bail!("[fn get_channel_manager_for_external_id()] No User found for external_id '{}'", external_id)
    }
}

fn create_conditions_for_external_id_filter(external_id: &str, match_type: MatchType) -> Vec<Condition> {
    let mut conditions_buffer = Vec::<Condition>::new();
    let mut conditions = Conditions(&mut conditions_buffer);
    conditions.add(Some(external_id.to_string()), "external_id", match_type);
    conditions_buffer
}

async fn get_channel_manager_by_user_dao(
    user_dao: User,
    wallet: PlainTextWallet,
    node: &str,
    serialize_streams_client_state_callback: Option<SerializationCallbackRefToClosureString>,
    message_data_store_for_msg_caching: Option<MessageDataStoreOptions>,
    throttle_sleep_time_millisecs: Option<u64>
) -> Result<ChannelManager<PlainTextWallet>>{
    let mut new_opt = ChannelManagerOptions::default();
    new_opt.throttle_sleep_time_millisecs = throttle_sleep_time_millisecs;
    new_opt.streams_client_state = Some(user_dao.streams_client_state.clone());
    new_opt.serialize_streams_client_state_callback = serialize_streams_client_state_callback;
    new_opt.message_data_store_for_msg_caching = message_data_store_for_msg_caching;
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