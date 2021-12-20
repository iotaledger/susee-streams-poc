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

async fn create_channel(channel_manager: &mut ChannelManagerPlainTextWallet) -> Result<()>{
    let announcement_link = channel_manager.create_announcement().await?;
    println_announcement_link(&announcement_link, "A channel has been created with the following announcement link");
    Ok(())
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

async fn send_keyload_message<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>) -> Result<()>
{
    let sub_msg_link_string = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
    let subscription_msg_link = Address::from_str(sub_msg_link_string)?;
    let pub_key_str = cli.matches.value_of(cli.arg_keys.subscription_pub_key).unwrap();
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

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    let arg_matches = get_arg_matches();
    let cli = ManagementConsoleCli::new(&arg_matches, &ARG_KEYS) ;
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
        create_channel(&mut channel_manager).await?
    } else if cli.matches.is_present(cli.arg_keys.subscription_link) {
        send_keyload_message(&mut channel_manager, &cli).await?
    } else {
        println!("[Management Console] You need to specify one of these options: --{} or --{}\n", cli.arg_keys.create_channel, cli.arg_keys.subscription_link);
        println_channel_status(&mut channel_manager ).await;
    }

    Ok(())
}

