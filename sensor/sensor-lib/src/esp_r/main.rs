use super::{
    cli::{
        ProcessingArgs,
        ArgKeys,
    },
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    }
};

use streams_tools::{
    subscriber_manager::get_public_key_str,
    DummyWallet,
    SubscriberManager
};

use iota_streams::app_channels::api::{
    tangle::{
        Address,
        Bytes,
        Subscriber,
    }
};

use core::str::FromStr;

use anyhow::Result;

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

pub async fn process_main_esp_rs(client_factory: TangleHttpClientFactory) -> Result<()> {
    println!("[Sensor] process_main() entry");
    let wallet = DummyWallet{};

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    println!("[Sensor] Creating HttpClient");
    let client = client_factory(None);
    println!("[Sensor] Creating subscriber");
    let mut subscriber= SubscriberManagerDummyWalletHttpClient::new(
        client,
        wallet,
        Some(String::from("user-state-sensor.bin")),
    ).await;

    println!("[Sensor] subscriber created");
    let mut show_subscriber_state = true;

    #[cfg(feature = "esp_idf")]
        print_heap_info();

    println!("[Sensor] subscribe_announcement_link");
    if ProcessingArgs::contains_key(ArgKeys::SUBSCRIBE_ANNOUNCEMENT_LINK) {
        let announcement_link_str = ProcessingArgs::get(ArgKeys::SUBSCRIBE_ANNOUNCEMENT_LINK);
        show_subscriber_state = false;
        subscribe_to_channel(announcement_link_str, &mut subscriber).await?
    }

    if ProcessingArgs::contains_key(ArgKeys::CONTENT_TO_SEND) {
        let content_to_send= ProcessingArgs::get(ArgKeys::CONTENT_TO_SEND);
        // send_content_as_msg(content_to_send.clone(), &mut subscriber).await?;
    }

    if ProcessingArgs::contains_key(ArgKeys::REGISTER_KEYLOAD_MSG) {
        let keyload_msg_link_str = ProcessingArgs::get(ArgKeys::REGISTER_KEYLOAD_MSG);
        show_subscriber_state = false;
        // register_keyload_msg(keyload_msg_link_str, &mut subscriber).await?
    }

    if show_subscriber_state {
        println_subscriber_status(&subscriber);
    }

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