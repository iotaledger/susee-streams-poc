// #![no_std]

use anyhow::Result;

mod cli;

use streams_tools::{SubscriberManagerPlainTextWallet, HttpClient, CaptureClient};

use cli::{
    SensorCli,
    ARG_KEYS,
    get_arg_matches,
};

use iota_streams::app_channels::api::tangle::{Address, Bytes};
use core::str::FromStr;

use std::{
    fs::File,
    path::Path,
    io::{
        Read,
        BufReader
    }
};
use iota_streams::app_channels::api::DefaultF;
use susee_tools::{SUSEE_CONST_SECRET_PASSWORD, get_wallet};
use clap::Values;

type ClientType = HttpClient<DefaultF>; // CaptureClient; //

type SubscriberManagerPlainTextWalletHttpClient = SubscriberManagerPlainTextWallet<ClientType>;

async fn send_file_content_as_msg(msg_file: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<Address>{
    let f = File::open(msg_file)?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    println!("[Main] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

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

async fn subscribe_to_channel(announcement_link_str: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
    let ann_address = Address::from_str(&announcement_link_str)?;
    let sub_msg_link = subscriber.subscribe(&ann_address).await?;

    let sub_msg_link_string = sub_msg_link.to_string();
    println!(
        "[Sensor] Subscription message link for subscriber_a: {}\n       Tangle Index: {:#}\n",
        sub_msg_link_string, sub_msg_link.to_msg_index()
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
        "wallet-management-console.txt"
    )?;

    println!("[Sensor] Using node '{}' for tangle connection", cli.node);

    //let client = HttpClient::new_from_url(&cli.node, None);
    let client = ClientType::new_from_url(&cli.node, None); //
    let mut subscriber= SubscriberManagerPlainTextWalletHttpClient::new(
        client,
        wallet,
        Some(String::from("user-state-sensor.bin")),
    ).await;

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        subscribe_to_channel(announcement_link_str, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        send_messages(files_to_send, &mut subscriber).await?
    }

    Ok(())
}
