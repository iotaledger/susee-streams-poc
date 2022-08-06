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

use crate::{
    std::{
        ClientType,
        SubscriberManagerPlainTextWalletHttpClient,
        sensor_manager::SensorManager,
        remote_manager::{
            RemoteManager,
            RemoteManagerOptions,
        },
    }
};

use streams_tools::client::http_client::HttpClientOptions;

pub async fn process_local_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    let wallet = get_wallet(
        &cli.matches,
        SUSEE_CONST_SECRET_PASSWORD,
        cli.arg_keys.base.wallet_file,
        "wallet-sensor.txt"
    )?;

    println!("[Sensor] Using node '{}' for tangle connection", cli.node);

    let mut http_client_options: Option<HttpClientOptions> = None;
    if let Some(tangle_proxy_url) = cli.matches.value_of(cli.arg_keys.tangle_proxy_url) {
        http_client_options = Some(HttpClientOptions{
            http_url: tangle_proxy_url
        });
    }

    let client = ClientType::new_from_url(&cli.node, http_client_options);
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

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        show_subscriber_state = false;
        SensorManager::clear_client_state(&mut subscriber).await?
    }

    if show_subscriber_state {
        SensorManager::println_subscriber_status(&subscriber);
    }

    Ok(())
}

pub async fn process_remote_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    let mut show_subscriber_state = cli.matches.is_present(cli.arg_keys.println_subscriber_status);

    let mut remote_manager_options: Option<RemoteManagerOptions> = None;
    if let Some(tangle_proxy_url) = cli.matches.value_of(cli.arg_keys.tangle_proxy_url) {
        remote_manager_options = Some(RemoteManagerOptions{
            http_url: tangle_proxy_url
        });
    }

    let remote_manager = RemoteManager::new(remote_manager_options);

    println!("[Sensor] Acting as remote sensor using {} as iota-bridge url", remote_manager.get_proxy_url());

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        println!("[Sensor] Sending subscribe_announcement_link command to remote sensor. announcement_link: {}", announcement_link_str);
        show_subscriber_state = false;
        remote_manager.subscribe_to_channel(announcement_link_str).await?
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        println!("[Sensor] Sending files_to_send command to remote sensor.");
        remote_manager.send_messages(files_to_send).await?
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        println!("[Sensor] Sending register_keyload_msg command to remote sensor. keyload_msg_link: {}", keyload_msg_link_str);
        show_subscriber_state = false;
        remote_manager.register_keyload_msg(keyload_msg_link_str).await?
    }

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        println!("[Sensor] Sending clear_client_state command to remote sensor.");
        remote_manager.clear_client_state().await?
    }

    if show_subscriber_state {
        remote_manager.println_subscriber_status().await?;
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
