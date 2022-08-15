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
    }
};

use streams_tools::{
    http_client::HttpClientOptions,
    remote::remote_sensor::{
        RemoteSensor,
        RemoteSensorOptions,
    },
};

pub async fn process_local_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    let wallet = get_wallet(
        &cli.matches,
        SUSEE_CONST_SECRET_PASSWORD,
        cli.arg_keys.base.wallet_file,
        "wallet-sensor.txt"
    )?;

    println!("[Sensor] Using node '{}' for tangle connection", cli.node);

    let mut http_client_options: Option<HttpClientOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        http_client_options = Some(HttpClientOptions{
            http_url: iota_bridge_url
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

    let mut remote_manager_options: Option<RemoteSensorOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_manager_options = Some(RemoteSensorOptions {
            http_url: iota_bridge_url,
            command_fetch_wait_seconds: 5,
        });
    }

    let remote_manager = RemoteSensor::new(remote_manager_options);

    println!("[Sensor] Acting as remote sensor using {} as iota-bridge url", remote_manager.get_proxy_url());

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        println!("[Sensor] Sending subscribe_announcement_link command to remote sensor. announcement_link: {}", announcement_link_str);
        show_subscriber_state = false;
        let confirm = remote_manager.subscribe_to_channel(announcement_link_str).await?;
        println!("[Sensor] Remote sensor confirmed Subscription: {}", confirm);
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let mut files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        if let Some(first_file) = files_to_send.nth(0) {
            println!("[Sensor] Sending files_to_send command to remote sensor.");
            let confirm = remote_manager.send_messages(first_file).await?;
            println!("[Sensor] Remote sensor confirmed files_to_send: {}", confirm);
        } else {
            println!("[Sensor] WARNING: Could not find any filename in files_to_send list.");
        }
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        println!("[Sensor] Sending register_keyload_msg command to remote sensor. keyload_msg_link: {}", keyload_msg_link_str);
        show_subscriber_state = false;
        let confirm = remote_manager.register_keyload_msg(keyload_msg_link_str).await?;
        println!("[Sensor] Remote sensor confirmed KeyloadRegistration: {}", confirm);
    }

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        println!("[Sensor] Sending clear_client_state command to remote sensor.");
        let confirm = remote_manager.clear_client_state().await?;
        println!("[Sensor] Remote sensor confirmed ClearClientState: {}", confirm);
    }

    if show_subscriber_state {
        let confirm = remote_manager.println_subscriber_status().await?;
        println!("[Sensor] Remote sensor SubscriberStatus: {}", confirm);
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
