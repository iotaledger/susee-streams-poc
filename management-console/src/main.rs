mod cli;

use cli::{
    ManagementConsoleCli,
    ARG_KEYS,
    get_arg_matches,
};

use streams_tools::{
    ChannelManagerPlainTextWallet,
    ChannelManager,
    channel_manager::SubscriberData
};

use susee_tools::{
    get_wallet,
    SUSEE_CONST_SECRET_PASSWORD
};

use iota_streams::{
    app_channels::api::tangle::Address,
    core::prelude::hex,
};

use core::str::FromStr;

use anyhow::Result;
use streams_tools::remote::remote_sensor::{RemoteSensorOptions, RemoteSensor};

fn println_announcement_link(link: &Address, comment: &str) {
    println!(
        "[Management Console] {}:
                     Announcement Link: {}
                          Tangle Index: {:#}\n",
        comment,
        link.to_string(),
        link.to_msg_index()
    );
}

async fn create_channel(channel_manager: &mut ChannelManagerPlainTextWallet) -> Result<Address>{
    let announcement_link = channel_manager.create_announcement().await?;
    println_announcement_link(&announcement_link, "A channel has been created with the following announcement link");
    Ok(announcement_link)
}

async fn println_channel_status<'a> (channel_manager: &mut ChannelManagerPlainTextWallet)
{
    let mut channel_exists = false;
    if let Some(author) = &channel_manager.author {
        match author.announcement_link() {
            Some(link) => {
                println_announcement_link(link, "A channel with the following details has already been announced");
                channel_exists = true
            },
            _ => {},
        }
    }
    if !channel_exists {
        println!("[Management Console] No existing channel found.");
    }
}


async fn init_sensor<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>) -> Result<()>{
    println!("[Management Console] Initializing remote sensor");
    let announcement_link = create_channel(channel_manager).await?;

    let mut remote_manager_options: Option<RemoteSensorOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_manager_options = Some(RemoteSensorOptions {
            http_url: iota_bridge_url,
            command_fetch_wait_seconds: 5,
        });
    }
    let remote_manager = RemoteSensor::new(remote_manager_options);
    println!("[Management Console] Using {} as iota-bridge url", remote_manager.get_proxy_url());

    println!("[Management Console] Sending subscribe_announcement_link command to remote sensor.\n");
    let subscription_confirm = remote_manager.subscribe_to_channel(announcement_link.to_string().as_str()).await?;

    println!("
[Management Console] Received confirmation for successful Subscription from remote sensor.
                     Creating keyload_message for
                            subscription: {}
                            public key: {}\n",
             subscription_confirm.subscription_link,
             subscription_confirm.pup_key,
    );
    let keyload_msg_link = send_keyload_message(
        channel_manager,
        subscription_confirm.subscription_link.as_str(),
        subscription_confirm.pup_key.as_str()
    ).await?;

    println!("[Management Console] Sending register_keyload_msg command to remote sensor.\n");
    let _keyload_registration_confirm = remote_manager.register_keyload_msg(keyload_msg_link.to_string().as_str()).await?;
    println!("
[Management Console] Received confirmation for successful KeyloadRegistration from remote sensor.
                     =========> Sensor has been fully initialized <===========");
    Ok(())
}

async fn send_keyload_message_cli<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>) -> Result<Address>
{
    let sub_msg_link_string = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
    let pub_key_str = cli.matches.value_of(cli.arg_keys.subscription_pub_key).unwrap();
    send_keyload_message(channel_manager, sub_msg_link_string, pub_key_str).await
}

async fn send_keyload_message<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, sub_msg_link_string: &str, pub_key_str: &str) -> Result<Address>
{
    let subscription_msg_link = Address::from_str(sub_msg_link_string)?;
    let pub_key = hex::decode(pub_key_str).unwrap();
    let keyload_msg_link = channel_manager.add_subscribers(&vec![ SubscriberData {
        subscription_link: & subscription_msg_link,
        public_key: pub_key.as_slice()
    }]).await?;

    println!(
        "\
[Management Console] A keyload message has been created with the following keyload link:
                     Keyload link: {}
                     Tangle Index: {:#}
", keyload_msg_link.to_string(), keyload_msg_link.to_msg_index()
    );

    Ok(keyload_msg_link)
}

#[tokio::main]
async fn main() -> Result<()> {

    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = ManagementConsoleCli::new(&matches_and_options, &ARG_KEYS) ;
    let wallet = get_wallet(
        &cli.matches,
        SUSEE_CONST_SECRET_PASSWORD,
        cli.arg_keys.base.wallet_file,
        "wallet-management-console.txt"
    )?;

    println!("[Management Console] Using node '{}' for tangle connection", cli.node);

    let mut channel_manager = ChannelManager::new(
        cli.node,
        wallet,
        Some(String::from("user-state-management-console.bin"))
    ).await;

    if cli.matches.is_present(cli.arg_keys.create_channel) {
        create_channel(&mut channel_manager).await?;
    } else if cli.matches.is_present(cli.arg_keys.subscription_link) {
        send_keyload_message_cli(&mut channel_manager, &cli).await?;
    } else if cli.matches.is_present(cli.arg_keys.init_sensor) {
        init_sensor(&mut channel_manager, &cli).await?;
    } else if cli.matches.is_present(cli.arg_keys.println_channel_status) {
        println_channel_status(&mut channel_manager ).await;
    } else {
        println!("[Management Console] You need to specify one of these options: --{}, --{}, --{} or --{}\n",
                 cli.arg_keys.create_channel,
                 cli.arg_keys.subscription_link,
                 cli.arg_keys.init_sensor,
                 cli.arg_keys.println_channel_status,
        );
        // println_channel_status(&mut channel_manager ).await;
    }

    Ok(())
}

