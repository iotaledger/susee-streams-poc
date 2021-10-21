// #![no_std]

use anyhow::Result;

mod channel_manager;
mod subscriber_manager;
mod helpers;
mod capture_client;
mod cli;

use channel_manager::ChannelManager;
use subscriber_manager::SubscriberManager;
use cli::{
    Cli,
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

async fn send_file_content_as_msg(msg_file: &str, channel: &mut ChannelManager) -> Result<Address>{
    let f = File::open(msg_file)?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    println!("[Main] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

    channel.send_signed_packet(&Bytes(buffer.clone())).await
}

#[tokio::main]
async fn main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = Cli::new(&arg_matches) ;
    let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();

    println!("[Main] Using node '{}' for tangle connection", cli.node);

    for msg_file in files_to_send.clone() {
        if !Path::new(msg_file).exists(){
            panic!("Can not find message file '{}'", msg_file);
        }
    }
    let mut channel = ChannelManager::new(cli.node);
    
    let announcement_link = channel.create_announcement().await?;
    let ann_link_string = announcement_link.to_string();

    println!(
        "[Main] Announcement Link: {}\n       Tangle Index: {:#}\n",
        ann_link_string, announcement_link.to_msg_index()
    );

    let mut subscriber_a: SubscriberManager = SubscriberManager::new(cli.node);

    // In a real world use a subscriber would receive the announcement_link as a text from a website
    // api, email or similar
    let ann_address = Address::from_str(&ann_link_string)?;
    let sub_msg_link_a = subscriber_a.subscribe(&ann_address).await?;

    let sub_msg_link_a_string = sub_msg_link_a.to_string();
    println!(
        "[Main] Subscription message link for subscriber_a: {}\n       Tangle Index: {:#}\n",
        sub_msg_link_a_string, sub_msg_link_a.to_msg_index()
    );

    // The Subscribers will send their subscription messages as a text to the author via website
    // api, email or similar
    let subscription_msg_link_a = Address::from_str(&sub_msg_link_a_string)?;
    let keyload_msg_link = channel.add_subscribers(&vec![
        &subscription_msg_link_a,
    ]).await?;

    println!(
        "[Main] Keyload message link: {}\n       Tangle Index: {:#}\n",
        keyload_msg_link.to_string(), keyload_msg_link.to_msg_index()
    );

    for msg_file in files_to_send {
        let msg_link = send_file_content_as_msg(msg_file, &mut channel).await?;
        println!("[Main] Sent msg from file '{}': {}, tangle index: {:#}\n", msg_file, msg_link, msg_link.to_msg_index());
    }

    Ok(())
}
