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
    binary_persist::{
        BinaryPersist,
        Command,
        SubscribeToAnnouncement,
        RegisterKeyloadMessage,
        StartSendingMessages,
    },
    DummyWallet,
    SubscriberManager,
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
    anyhow,
    Result,
};
use std::{
    time::Duration,
    thread,
};
use streams_tools::binary_persist::{Subscription, SubscriberStatus};
use streams_tools::http::http_protocol_confirm::RequestBuilderConfirm;
use hyper::{
    Body,
    http::{
        Request,
        status,
    }
};

type ClientType = HttpClient;

type SubscriberManagerDummyWalletHttpClient = SubscriberManager<ClientType, DummyWallet>;

const IOTA_BRIDGE_URL: &str = env!("SENSOR_MAIN_POC_IOTA_BRIDGE_URL");

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

fn println_subscriber_status<'a> (
    subscriber_manager: &SubscriberManagerDummyWalletHttpClient,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let mut ret_val: Option<Request<Body>> = None;
    if let Some(subscriber) = &subscriber_manager.subscriber {
        if let Some(subscription_link) = subscriber_manager.subscription_link {
            println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");

            let mut previous_message_link= String::from("");
            if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
                previous_message_link = prev_msg_link.to_string();
            }

            ret_val = Some(
                confirm_req_builder.subscriber_status(
                    previous_message_link,
                    Subscription {
                        subscription_link: subscription_link.to_string(),
                        pup_key: get_public_key_str(subscriber),
                    })?
            );
        }
    }
    if ret_val.is_none() {
        println!("[Sensor] No subscription found.");
        let to_send = SubscriberStatus::default();
        ret_val = Some( confirm_req_builder.subscriber_status(to_send.previous_message_link, to_send.subscription)?);
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

    // hyper::http::Error::from(InvalidStatusCode::from(404).unwrap())
    ret_val.ok_or_else(|| {
        if let Err(e) = status::StatusCode::from_u16(404) {
            e.into()
        } else {
            panic!("Should never happen");
        }
    })
}

async fn clear_client_state<'a> (
    subscriber_manager: &mut SubscriberManagerDummyWalletHttpClient,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    subscriber_manager.clear_client_state().await.expect("subscriber_manager.clear_client_state() returned error");
    confirm_req_builder.clear_client_state()
}

pub async fn send_content_as_msg(
    message_key: String,
    subscriber: &mut SubscriberManagerDummyWalletHttpClient,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let message_bytes = get_message_bytes(Message::from(message_key.as_str()));
    log::info!("[fn send_content_as_msg] Sending {} bytes payload\n", message_bytes.len());
    log::debug!("[fn send_content_as_msg] - send_content_as_msg()] Message text: {}", std::str::from_utf8(message_bytes).expect("Could not deserialize message bytes to utf8 str"));
    let prev_message = subscriber.send_signed_packet(&Bytes(message_bytes.to_vec())).await.expect("subscriber.send_signed_packet() returned error");
    confirm_req_builder.send_message(prev_message.to_string())
}

async fn subscribe_to_channel(
    announcement_link_str: &str,
    subscriber_mngr: &mut SubscriberManagerDummyWalletHttpClient,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let ann_address = Address::from_str(&announcement_link_str).expect("Address::from_str() returned error");
    let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await.expect("subscriber_mngr::subscribe() returned error");

    let subscriber = subscriber_mngr.subscriber.as_ref().unwrap();

    println_subscription_details(
        &subscriber,
        &sub_msg_link,
        "New subscription",
        "Subscription",
    );

    confirm_req_builder.subscription(sub_msg_link.to_string(), get_public_key_str(subscriber))
}

async fn register_keyload_msg(
    keyload_msg_link_str: &str,
    subscriber_mngr: &mut SubscriberManagerDummyWalletHttpClient,
    confirm_req_builder: &RequestBuilderConfirm
) -> hyper::http::Result<Request<Body>>
{
    let keyload_msg_link = Address::from_str(&keyload_msg_link_str).expect("Address::from_str() returned error");
    subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[fn register_keyload_msg] register_keyload_msg err");

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &keyload_msg_link,
        "Keyload Message",
        "Keyload  msg",
    );

    confirm_req_builder.keyload_registration()
}

async fn process_command(command: Command, buffer: Vec<u8>) -> Result<Request<Body>>{
    let wallet = DummyWallet{};

    #[cfg(feature = "esp_idf")]
        let vfs_fat_handle = setup_vfs_fat_filesystem()?;

    log::debug!("[fn process_command]  Creating HttpClient");
    let client = HttpClient::new(Some(HttpClientOptions{ http_url: IOTA_BRIDGE_URL }));
    log::debug!("[fn process_command] Creating subscriber");
    let mut subscriber= SubscriberManagerDummyWalletHttpClient::new(
        client,
        wallet,
        Some(String::from(BASE_PATH) + "/user-state-sensor.bin"),
    ).await;

    log::debug!("[fn process_command]  subscriber created");

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    let confirm_req_builder = RequestBuilderConfirm::new(IOTA_BRIDGE_URL);
    let mut confirmation_request: Option<Request<Body>> = None;

    if command == Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK {
        let cmd_args = SubscribeToAnnouncement::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing SUBSCRIBE_ANNOUNCEMENT_LINK: {}", cmd_args.announcement_link);
        confirmation_request = Some(
            subscribe_to_channel(cmd_args.announcement_link.as_str(), &mut subscriber, &confirm_req_builder).await?
        );
    }

    if command == Command::START_SENDING_MESSAGES {
        let cmd_args = StartSendingMessages::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing START_SENDING_MESSAGES: {}", cmd_args.message_template_key);
        confirmation_request = Some(
            send_content_as_msg(cmd_args.message_template_key, &mut subscriber, &confirm_req_builder).await?
        );
    }

    if command == Command::REGISTER_KEYLOAD_MESSAGE {
        let cmd_args = RegisterKeyloadMessage::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_command]  processing REGISTER_KEYLOAD_MESSAGE: {}", cmd_args.keyload_msg_link);
        confirmation_request = Some(
            register_keyload_msg(cmd_args.keyload_msg_link.as_str(), &mut subscriber, &confirm_req_builder ).await?
        );
    }

    if command == Command::PRINTLN_SUBSCRIBER_STATUS {
        log::info!("[fn process_command]  PRINTLN_SUBSCRIBER_STATUS");
        confirmation_request = Some(
            println_subscriber_status(&subscriber, &confirm_req_builder)?
        );
    }

    if command == Command::CLEAR_CLIENT_STATE {
        log::info!("[fn process_command]  =========> processing CLEAR_CLIENT_STATE <=========");

        confirmation_request = Some(
            clear_client_state(&mut subscriber, &confirm_req_builder).await?
        );
    }

    #[cfg(feature = "esp_idf")]
    {
        log::debug!("[fn process_command]  Safe subscriber client_status to disk");
        subscriber.safe_client_status_to_disk().await?;
        log::debug!("[fn process_command]  drop_vfs_fat_filesystem");
        drop_vfs_fat_filesystem(vfs_fat_handle)?;
    }

    confirmation_request.ok_or(anyhow!("No confirmation_request received"))
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

    log::info!("[fn process_main_esp_rs] Using iota-bridge url: {}", IOTA_BRIDGE_URL);
    let command_fetcher = CommandFetcher::new(Some(CommandFetcherOptions{ http_url: IOTA_BRIDGE_URL }));

    loop {
        if let Ok((command, buffer)) = command_fetcher.fetch_next_command() {
            if command != Command::NO_COMMAND {
                log::info!("[fn process_main_esp_rs] Starting process_command for command: {}.", command);
                match process_command(command, buffer).await {
                    Ok(confirmation_request) => {
                        // TODO: Retries in case of errors could be useful
                        log::debug!("[fn process_main_esp_rs] Calling command_fetcher.send_confirmation for confirmation_request");
                        command_fetcher.send_confirmation(confirmation_request).await?;
                    },
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