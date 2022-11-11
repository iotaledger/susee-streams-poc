use anyhow::{
    Result,
    bail,
};

use streams_tools::{
    channel_manager::{
        ChannelManagerOptions,
    },
    ChannelManager,
    helpers::{
        get_channel_id_from_link,
        SerializationCallbackRefToClosure
    },
    PlainTextWallet,
    UserDataStore,
    user_manager::dao::User
};

use susee_tools::{
    get_wallet_filename,
    SUSEE_CONST_SECRET_PASSWORD
};

use std::{
    time::SystemTime,
    hash::{
        Hash,
        Hasher
    },
    collections::hash_map::DefaultHasher
};

use crate::cli::ManagementConsoleCli;


async fn get_initial_channel_manager<'a>(user_store: &UserDataStore, cli: &ManagementConsoleCli<'a>) -> Result<ChannelManager<PlainTextWallet>> {
    let mut new_opt = ChannelManagerOptions::default();
    let wallet = get_wallet(cli, None)?;
    new_opt.serialize_user_state_callback = Some(
        user_store.get_serialization_callback(wallet.seed_derivation_phrase.as_ref().unwrap().as_str())
    );

    Ok( ChannelManager::new(
        cli.node,
        wallet,
        Some(new_opt)
    ).await)
}

async fn get_channel_manager_for_cli_arg_channel_starts_with<'a>(user_store: &mut UserDataStore, cli: &ManagementConsoleCli<'a>, update_user_on_exit: bool) -> Result<ChannelManager<PlainTextWallet>> {
    if let Some(channel_starts_with) = cli.matches.value_of(cli.arg_keys.channel_starts_with) {
        if let Ok((user_dao, serialize_user_state_callback)) = user_store.search_user_state(channel_starts_with) {
            let wallet = get_wallet(cli, Some(&user_dao))?;
            let user_state_callback = if update_user_on_exit {
                Some(serialize_user_state_callback)
            } else {
                None
            };
            get_channel_manager_by_user_dao(
                user_dao,
                user_state_callback,
                wallet,
                cli.node
            ).await
        } else {
            bail!("Could not find matching Streams channel for ID starting with '{}'", channel_starts_with)
        }
    } else {
        bail!("To use this CLI command you need to specify CLI argument '--{}'", cli.arg_keys.channel_starts_with)
    }
}

async fn get_channel_manager_for_channel_id<'a>(channel_id: &str, user_store: &mut UserDataStore, cli: &ManagementConsoleCli<'a>) -> Result<ChannelManager<PlainTextWallet>> {
    let (user_dao, serialize_user_state_callback) = user_store.get_user_state(channel_id)?;
    let wallet = get_wallet(cli, Some(&user_dao))?;
    get_channel_manager_by_user_dao(
        user_dao,
        Some(serialize_user_state_callback),
        wallet,
        cli.node
    ).await
}

async fn get_channel_manager_by_user_dao(user_dao: User, serialize_user_state_callback: Option<SerializationCallbackRefToClosure>, wallet: PlainTextWallet, node: &str) -> Result<ChannelManager<PlainTextWallet>>{
    let mut new_opt = ChannelManagerOptions::default();
    new_opt.user_state = Some(user_dao.streams_user_state.clone());
    new_opt.serialize_user_state_callback = serialize_user_state_callback;
    Ok( ChannelManager::new(
        node,
        wallet,
        Some(new_opt)
    ).await)
}

fn get_wallet(cli: &ManagementConsoleCli, user_dao: Option<&User>) -> Result<PlainTextWallet>{
    let wallet_filename= get_wallet_filename(
        &cli.matches,
        cli.arg_keys.base.wallet_file,
        "wallet-management-console.txt",
    )?;

    let seed_derivation_phrase = if let Some(user) = user_dao {
        user.seed_derivation_phrase.clone()
    } else {
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("{}", hasher.finish())
    };

    Ok(PlainTextWallet::new(
        SUSEE_CONST_SECRET_PASSWORD,
        Some(wallet_filename.as_str()),
        Some(seed_derivation_phrase),
    ))
}

pub async fn create_channel_manager<'a>(user_store: &mut UserDataStore, cli: &ManagementConsoleCli<'a>) -> Option<ChannelManager<PlainTextWallet>> {
    let mut ret_val = None;
    if cli.matches.is_present(cli.arg_keys.create_channel)
        || cli.matches.is_present(cli.arg_keys.init_sensor) {
        ret_val = Some(get_initial_channel_manager(user_store, cli).await.unwrap());
    }
    else if cli.matches.is_present(cli.arg_keys.subscription_link) {
        let sub_msg_link_str = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
        if let Some(channel_id ) = get_channel_id_from_link(sub_msg_link_str) {
            ret_val = Some(get_channel_manager_for_channel_id(channel_id.as_str(), user_store, cli).await.unwrap());
        } else {
            println!("[Management Console] Could not parse channel_id from CLI argument '--{}'. Argument value is {}",
                     cli.arg_keys.subscription_link,
                     sub_msg_link_str,
            )
        }
    } else if cli.matches.is_present(cli.arg_keys.println_channel_status) {
        ret_val = Some(get_channel_manager_for_cli_arg_channel_starts_with(user_store, cli, false).await.unwrap());
    }

    ret_val
}