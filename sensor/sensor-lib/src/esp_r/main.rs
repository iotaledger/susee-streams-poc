use super::{
    command_fetcher::{
        CommandFetcher,
    },
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    },
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

pub type TangleHttpClientFactory = fn(options: Option<HttpClientOptions>) -> HttpClient;

async fn send_content_as_msg(content_to_send: &str, subscriber: &mut SubscriberManagerDummyWalletHttpClient) -> Result<Address>{
    let buffer = String::from(content_to_send).into_bytes();
    println!("[Sensor] Sending {} bytes payload\n", buffer.len());

    subscriber.send_signed_packet(&Bytes(buffer.clone())).await
    // Ok(Address::default())
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

fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerDummyWalletHttpClient)
{
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
        println!("[Sensor] Prev msg link: {}", prev_msg_link);
    }
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
    subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[Sensor] register_keyload_msg err");

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &keyload_msg_link,
        "Keyload Message",
        "Keyload  msg",
    );

    Ok(())
}

async fn process_command(command: Command, buffer: Vec<u8>, client_factory: TangleHttpClientFactory) -> Result<()>{
    let wallet = DummyWallet{};

    #[cfg(feature = "esp_idf")]
        let vfs_fat_handle = setup_vfs_fat_filesystem()?;

    println!("[Sensor] Creating HttpClient");
    let client = client_factory(None);
    println!("[Sensor] Creating subscriber");
    let mut subscriber= SubscriberManagerDummyWalletHttpClient::new(
        client,
        wallet,
        Some(String::from(BASE_PATH) + "/user-state-sensor.bin"),
    ).await;

    println!("[Sensor] subscriber created");

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    println!("[Sensor] check for command SUBSCRIBE_TO_ANNOUNCEMENT_LINK");
    if command != Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK {
        let cmd_args = SubscribeToAnnouncement::try_from_bytes(buffer.as_slice())?;
        println!("[Sensor] SUBSCRIBE_ANNOUNCEMENT_LINK: {}", cmd_args.announcement_link);
        subscribe_to_channel(cmd_args.announcement_link.as_str(), &mut subscriber).await?
    }

    println!("[Sensor] check for command START_SENDING_MESSAGES");
    if command != Command::START_SENDING_MESSAGES {
        let cmd_args = StartSendingMessages::try_from_bytes(buffer.as_slice())?;
        println!("[Sensor] START_SENDING_MESSAGES: {}", cmd_args.message_template_key);
        send_content_as_msg(cmd_args.message_template_key.as_str(), &mut subscriber).await?;
    }

    println!("[Sensor] check for command REGISTER_KEYLOAD_MESSAGE");
    if command != Command::REGISTER_KEYLOAD_MESSAGE {
        let cmd_args = RegisterKeyloadMessage::try_from_bytes(buffer.as_slice())?;
        println!("[Sensor] REGISTER_KEYLOAD_MESSAGE: {}", cmd_args.keyload_msg_link);
        register_keyload_msg(cmd_args.keyload_msg_link.as_str(), &mut subscriber).await?
    }

    println!("[Sensor] check for command PRINTLN_SUBSCRIBER_STATUS");
    if command != Command::PRINTLN_SUBSCRIBER_STATUS {
        println_subscriber_status(&subscriber);
    }

    #[cfg(feature = "esp_idf")]
        drop_vfs_fat_filesystem(vfs_fat_handle);

    Ok(())
}

pub async fn process_main_esp_rs(client_factory: TangleHttpClientFactory) -> Result<()> {
    println!("[Sensor] process_main() entry");

    let command_fetch_wait_seconds = 5;

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    #[cfg(feature = "wifi")]
        println!("[Sensor] init_wifi");
    #[cfg(feature = "wifi")]
        let (wifi_hdl, client_settings) = init_wifi()?;

    let command_fetcher = CommandFetcher::new(None);

    loop {
        if let Ok((command, buffer)) = command_fetcher.fetch_next_command() {
            if command != Command::NO_COMMAND {
                process_command(command, buffer, client_factory);
            }
        } else {
            println!("[Sensor] command_fetcher.fetch_next_command() failed.");
        }

        for s in 0..command_fetch_wait_seconds {
            println!("Fetching next command in {} secs", command_fetch_wait_seconds - s);
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(feature = "wifi")]
        drop(wifi_hdl);

    Ok(())
}

#[cfg(feature = "esp_idf")]
fn print_heap_info() {
    unsafe {
        // esp_idf_sys::heap_caps_print_heap_info(
        //     esp_idf_sys::MALLOC_CAP_8BIT
        // );

        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        println!("heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }
}