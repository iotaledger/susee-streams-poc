use super::{
    command_fetcher::{
        CommandFetcher,
        CommandFetcherOptions,
    },
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    },
};

use payloads::{
    Message,
    get_message_bytes,
};

#[cfg(feature = "wifi")]
use super::{
    wifi_utils::init_wifi,
};

#[cfg(feature = "esp_idf")]
use super::{
    vfs_fat_fs_tools::{
        setup_vfs_fat_filesystem,
        drop_vfs_fat_filesystem,
        BASE_PATH,
    }
};

use streams_tools::{
    subscriber_manager::get_public_key_str,
    binary_persist_command::{
        Command,
        SubscribeToAnnouncement,
        RegisterKeyloadMessage,
        StartSendingMessages,
    },
    DummyWallet,
    SubscriberManager,
    BinaryPersist
};

use iota_streams::app_channels::api::{
    tangle::{
        Address,
        Bytes,
        Subscriber,
    }
};

use core::str::FromStr;

use anyhow::{
    Result,
};
use std::{
    time::Duration,
    thread,
};


type ClientType = HttpClient;

type SubscriberManagerDummyWalletHttpClient = SubscriberManager<ClientType, DummyWallet>;

const TANGLE_PROXY_URL: &str = env!("SENSOR_MAIN_POC_TANGLE_PROXY_URL");

#[cfg(feature = "esp_idf")]
fn print_heap_info() {
    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        log::info!("[fn print_heap_info] heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }
}

fn println_subscription_details(subscriber: &Subscriber<ClientType>, subscription_link: &Address, comment: &str, key_name: &str) {
    let public_key = get_public_key_str(subscriber);
    println!(
        "[Sensor] {}:
         {} Link:     {}
              Tangle Index:     {:#}
         Subscriber public key: {}\n",
        comment,
        key_name,
        subscription_link.to_string(),
        subscription_link.to_msg_index(),
        public_key,
    );
}

fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerDummyWalletHttpClient) {
    let mut subscription_exists = false;
    if let Some(subscriber) = &subscriber_manager.subscriber {
        if let Some(subscription_link) = subscriber_manager.subscription_link {
            println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");
        }
        subscription_exists = true;
    }
    if !subscription_exists {
        println!("[Sensor] No subscription found.");
    }

    if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
        println!(
            "[Sensor] Previous message:
         Prev msg link:     {}
             Tangle Index:     {:#}",
            prev_msg_link.to_string(),
            prev_msg_link.to_msg_index()
        );
    }
}

async fn clear_client_state<'a> (subscriber_manager: &mut SubscriberManagerDummyWalletHttpClient) -> Result<()> {
    subscriber_manager.clear_client_state().await?;
    Ok(())
}

pub async fn send_content_as_msg(message_key: String, subscriber: &mut SubscriberManagerDummyWalletHttpClient) -> Result<Address>{
    let message_bytes = get_message_bytes(Message::from(message_key.as_str()));
    log::info!("[fn send_content_as_msg] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn send_content_as_msg] - send_content_as_msg()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    subscriber.send_signed_packet(&Bytes(message_bytes.to_vec())).await
}

async fn subscribe_to_channel(announcement_link_str: &str, subscriber_mngr: &mut SubscriberManagerDummyWalletHttpClient) -> Result<()> {
    let ann_address = Address::from_str(&announcement_link_str)?;
    let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await?;

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &sub_msg_link,
        "New subscription",
        "Subscription",
    );

    Ok(())
}

async fn register_keyload_msg(keyload_msg_link_str: &str, subscriber_mngr: &mut SubscriberManagerDummyWalletHttpClient) -> Result<()> {
    let keyload_msg_link = Address::from_str(&keyload_msg_link_str)?;
    subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[fn register_keyload_msg] register_keyload_msg err");

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &keyload_msg_link,
        "Keyload Message",
        "Keyload  msg",
    );

    Ok(())
}

async fn process_command(command: Command, buffer: Vec<u8>) -> Result<()>{
    let wallet = DummyWallet{};

    #[cfg(feature = "esp_idf")]
        let vfs_fat_handle = setup_vfs_fat_filesystem()?;

    log::debug!("[fn process_command]  Creating HttpClient");
    let client = HttpClient::new(Some(HttpClientOptions{ http_url: TANGLE_PROXY_URL }));
    log::debug!("[fn process_command] Creating subscriber");
    let mut subscriber= SubscriberManagerDummyWalletHttpClient::new(
        client,
        wallet,
        Some(String::from(BASE_PATH) + "/user-state-sensor.bin"),
    ).await;

    log::debug!("[fn process_command]  subscriber created");

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    if command == Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK {
        let cmd_args = SubscribeToAnnouncement::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing SUBSCRIBE_ANNOUNCEMENT_LINK: {}", cmd_args.announcement_link);
        subscribe_to_channel(cmd_args.announcement_link.as_str(), &mut subscriber).await?
    }

    if command == Command::START_SENDING_MESSAGES {
        let cmd_args = StartSendingMessages::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing START_SENDING_MESSAGES: {}", cmd_args.message_template_key);
        send_content_as_msg(cmd_args.message_template_key, &mut subscriber).await?;
    }

    if command == Command::REGISTER_KEYLOAD_MESSAGE {
        let cmd_args = RegisterKeyloadMessage::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing REGISTER_KEYLOAD_MESSAGE: {}", cmd_args.keyload_msg_link);
        register_keyload_msg(cmd_args.keyload_msg_link.as_str(), &mut subscriber).await?
    }

    if command == Command::PRINTLN_SUBSCRIBER_STATUS {
        log::info!("[fn process_command]  PRINTLN_SUBSCRIBER_STATUS");
        println_subscriber_status(&subscriber);
    }

    if command == Command::CLEAR_CLIENT_STATE {
        log::info!("[fn process_command]  =========> processing CLEAR_CLIENT_STATE <=========");
        clear_client_state(&mut subscriber).await?;
    }

    #[cfg(feature = "esp_idf")]
    {
        log::debug!("[fn process_command]  Safe subscriber client_status to disk");
        subscriber.safe_client_status_to_disk().await?;
        log::debug!("[fn process_command]  drop_vfs_fat_filesystem");
        drop_vfs_fat_filesystem(vfs_fat_handle)?;
    }

    Ok(())
}

pub async fn process_main_esp_rs() -> Result<()> {
    log::debug!("[fn process_main_esp_rs] process_main() entry");

    let command_fetch_wait_seconds = 5;

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    #[cfg(feature = "wifi")]
        log::debug!("[fn process_main_esp_rs] init_wifi");
    #[cfg(feature = "wifi")]
        let (_wifi_hdl, _client_settings) = init_wifi()?;

    log::info!("[fn process_main_esp_rs] Using iota-bridge url: {}", TANGLE_PROXY_URL);
    let command_fetcher = CommandFetcher::new(Some(CommandFetcherOptions{ http_url: TANGLE_PROXY_URL }));

    loop {
        if let Ok((command, buffer)) = command_fetcher.fetch_next_command() {
            if command != Command::NO_COMMAND {
                log::info!("[fn process_main_esp_rs] Starting process_command for command: {}.", command);
                match process_command(command, buffer).await {
                    Ok(_) => {},
                    Err(err) => {
                        log::error!("[fn process_main_esp_rs] process_command() returned error: {}", err);
                    }
                };
            } else {
                log::info!("[fn process_main_esp_rs] Received Command::NO_COMMAND.");
            }
        } else {
            log::error!("[fn process_main_esp_rs] command_fetcher.fetch_next_command() failed.");
        }

        for s in 0..command_fetch_wait_seconds {
            println!("Fetching next command in {} secs", command_fetch_wait_seconds - s);
            thread::sleep(Duration::from_secs(1));
        }
    }
}