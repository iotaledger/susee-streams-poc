use super::cli::{
    SensorCli,
    ARG_KEYS,
    get_arg_matches,
};

use susee_tools::{
    SUSEE_CONST_SECRET_PASSWORD,
    get_wallet
};

use anyhow::Result;

use crate::std::{
    ClientType,
    SubscriberManagerPlainTextWalletHttpClient,
    sensor_manager::SensorManager,
    remote_manager::RemoteManager,
};

pub async fn process_local_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    let wallet = get_wallet(
        &cli.matches,
        SUSEE_CONST_SECRET_PASSWORD,
        cli.arg_keys.base.wallet_file,
        "wallet-sensor.txt"
    )?;

    println!("[Sensor] Using node '{}' for tangle connection", cli.node);

    //let client = HttpClient::new_from_url(&cli.node, None);
    let client = ClientType::new_from_url(&cli.node, None); //
    let mut subscriber= SubscriberManagerPlainTextWalletHttpClient::new(
        client,
        wallet,
        Some(String::from("user-state-sensor.bin")),
    ).await;

    let mut show_subscriber_state = true;

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        show_subscriber_state = false;
        SensorManager::subscribe_to_channel(announcement_link_str, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        SensorManager::send_messages(files_to_send, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        show_subscriber_state = false;
        SensorManager::register_keyload_msg(keyload_msg_link_str, &mut subscriber).await?
    }

    if show_subscriber_state {
        SensorManager::println_subscriber_status(&subscriber);
    }

    Ok(())
}

pub async fn process_remote_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    println!("[Sensor] Sending command to remote sensor using url {}", RemoteManager::get_proxy_url());

    let mut show_subscriber_state = true;

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        show_subscriber_state = false;
        RemoteManager::subscribe_to_channel(announcement_link_str).await?
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        RemoteManager::send_messages(files_to_send).await?
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        show_subscriber_state = false;
        RemoteManager::register_keyload_msg(keyload_msg_link_str).await?
    }

    if show_subscriber_state {
        RemoteManager::println_subscriber_status().await;
    }

    Ok(())
}


pub async fn process_main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = SensorCli::new(&arg_matches, &ARG_KEYS) ;

    if cli.matches.is_present(cli.arg_keys.act_as_remote_control) {
        process_remote_sensor(cli).await
    } else {
        process_local_sensor(cli).await
    }
}
