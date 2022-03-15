// #![no_std]

mod cli;

use cli::{
    SensorCli,
    ARG_KEYS,
    get_arg_matches,
};

use streams_tools::{
    SubscriberManagerPlainTextWallet,
    HttpClient,
    subscriber_manager::get_public_key_str,
};

use susee_tools::{
    SUSEE_CONST_SECRET_PASSWORD,
    get_wallet
};

use iota_streams::app_channels::api::{
    DefaultF,
    tangle::{
        Address,
        Bytes,
        Subscriber,
    }
};

use core::str::FromStr;

use std::{
    fs::File,
    path::Path,
    io::{
        Read,
        BufReader
    }
};

use anyhow::Result;

use clap::Values;

type ClientType = HttpClient;

type SubscriberManagerPlainTextWalletHttpClient = SubscriberManagerPlainTextWallet<ClientType>;

async fn send_file_content_as_msg(msg_file: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<Address>{
    let f = File::open(msg_file)?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    println!("[Sensor] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

    subscriber.send_signed_packet(&Bytes(buffer.clone())).await
}

async fn send_messages(files_to_send: Values<'_>, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()>{
    for msg_file in files_to_send.clone() {
        if !Path::new(msg_file).exists(){
            panic!("[Sensor] Can not find message file '{}'", msg_file);
        }
    }
    for msg_file in files_to_send {
        let msg_link = send_file_content_as_msg(msg_file, subscriber).await?;
        println!("[Sensor] Sent msg from file '{}': {}, tangle index: {:#}\n", msg_file, msg_link, msg_link.to_msg_index());
    }

    Ok(())
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

fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerPlainTextWalletHttpClient)
{
    let mut subscription_exists = false;
    if let Some(subscriber) = &subscriber_manager.subscriber {
        if let Some(subscription_link) = subscriber_manager.subscription_link {
            println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");
        }
        subscription_exists = true;
    }
    if !subscription_exists {
        println!("[Sensor] No existing subscription message found.");
    }

    if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
        println!("[Sensor] The last previously used message link is: {}", prev_msg_link);
    }
}

async fn subscribe_to_channel(announcement_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
    let ann_address = Address::from_str(&announcement_link_str)?;
    let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await?;

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &sub_msg_link,
        "A subscription with the following details has been created",
        "Subscription",
    );

    Ok(())
}
async fn register_keyload_msg(keyload_msg_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
    let keyload_msg_link = Address::from_str(&keyload_msg_link_str)?;
    subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[Sensor] Error while registering keyload msg");

    println_subscription_details(
        &subscriber_mngr.subscriber.as_ref().unwrap(),
        &keyload_msg_link,
        "Messages will be send in the branch defined by the following keyload message",
        "Keyload  msg",
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = SensorCli::new(&arg_matches, &ARG_KEYS) ;
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
        subscribe_to_channel(announcement_link_str, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        send_messages(files_to_send, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        show_subscriber_state = false;
        register_keyload_msg(keyload_msg_link_str, &mut subscriber).await?
    }

    if show_subscriber_state {
        println_subscriber_status(&subscriber);
    }


    Ok(())
}
