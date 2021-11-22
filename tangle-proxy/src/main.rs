use anyhow::Result;

mod cli;

use streams_tools::{
    ChannelManagerPlainTextWallet,
    SubscriberManager,
    FileStreamClient
};
use susee_tools::{
    get_wallet,
    SUSEE_CONST_SECRET_PASSWORD
};

use cli::{
    TangleProxyCli,
    ARG_KEYS,
    get_arg_matches,
};

use iota_streams::app_channels::api::tangle::{Address};
use core::str::FromStr;

use iota_streams::app_channels::api::DefaultF;

#[tokio::main]
async fn main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = TangleProxyCli::new(&arg_matches, &ARG_KEYS) ;
    let wallet = get_wallet(
        &cli.matches,
        SUSEE_CONST_SECRET_PASSWORD,
        cli.arg_keys.base.wallet_file,
        "wallet-management-console.txt"
    )?;

    println!("[Main] Using node '{}' for tangle connection", cli.node);

    let mut channel = ChannelManagerPlainTextWallet::new(
        cli.node,
        wallet,
        Some(String::from("sensor-state-management-console.bin"))
    ).await;

    let announcement_link = channel.create_announcement().await?;
    let ann_link_string = announcement_link.to_string();

    println!(
        "[Main] Announcement Link: {}\n       Tangle Index: {:#}\n",
        ann_link_string, announcement_link.to_msg_index()
    );

    let mut subscriber_a: SubscriberManager<FileStreamClient<DefaultF>> = SubscriberManager::new(cli.node);

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

    Ok(())
}
