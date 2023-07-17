use core::str::FromStr;

use anyhow::{
    Result,
    bail,
};

use iota_streams::{
    app_channels::api::tangle::Address,
    core::prelude::hex,
};

use streams_tools::{
    channel_manager::{
        SubscriberData,
    },
    subscriber_manager::println_maximum_initialization_cnt_reached_warning,
    ChannelManagerPlainTextWallet,
    multi_channel_management::{
        MultiChannelManagerOptions,
        get_initial_channel_manager,
        get_channel_manager_for_channel_id,
        get_channel_manager_for_channel_starts_with
    },
    remote::remote_sensor::{
        RemoteSensor,
        RemoteSensorOptions,
    },
    UserDataStore,
    binary_persist::INITIALIZATION_CNT_MAX_VALUE,
    explorer::{
        run_explorer_api_server,
        ExplorerOptions,
    },
    helpers::get_channel_id_from_link,
    dao_helpers::DbFileBasedDaoManagerOptions,
};

use susee_tools::{
    SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC,
    SUSEE_CONST_SECRET_PASSWORD,
    get_wallet_filename
};

use cli::{
    ARG_KEYS,
    get_arg_matches,
    ManagementConsoleCli,
};

mod cli;

pub async fn create_channel_manager<'a>(user_store: &mut UserDataStore, cli: &ManagementConsoleCli<'a>) -> Option<ChannelManagerPlainTextWallet> {
    let mut ret_val = None;
    let options = get_multi_channel_manager_options(cli).ok()?;
    if cli.matches.is_present(cli.arg_keys.create_channel)
        || cli.matches.is_present(cli.arg_keys.init_sensor) {
        ret_val = Some(get_initial_channel_manager(user_store, &options).await.unwrap());
    }
    else if cli.matches.is_present(cli.arg_keys.subscription_link) {
        let sub_msg_link_str = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
        if let Some(channel_id ) = get_channel_id_from_link(sub_msg_link_str) {
            ret_val = Some(get_channel_manager_for_channel_id(channel_id.as_str(), user_store, &options).await.unwrap());
        } else {
            println!("[Management Console] Could not parse channel_id from CLI argument '--{}'. Argument value is {}",
                     cli.arg_keys.subscription_link,
                     sub_msg_link_str,
            )
        }
    } else if cli.matches.is_present(cli.arg_keys.println_channel_status) {
        ret_val = Some(get_channel_manager_for_cli_arg_channel_starts_with(user_store, &options, cli, false).await.unwrap());
    }

    ret_val
}

fn get_management_console_wallet_filename<'a>(cli: &ManagementConsoleCli<'a>) -> Result<String> {
    get_wallet_filename(
        &cli.matches,
        cli.arg_keys.base.wallet_file,
        "wallet-management-console.txt",
    )
}

fn get_multi_channel_manager_options<'a>(cli: &ManagementConsoleCli<'a>) -> Result<MultiChannelManagerOptions> {
    let wallet_filename= get_management_console_wallet_filename(cli)?;
    Ok(MultiChannelManagerOptions{
        iota_node_url: cli.node.to_string(),
        wallet_filename,
        streams_user_serialization_password: SUSEE_CONST_SECRET_PASSWORD.to_string()
    })
}

pub async fn get_channel_manager_for_cli_arg_channel_starts_with<'a>(
    user_store: &mut UserDataStore,
    options: &MultiChannelManagerOptions,
    cli: &ManagementConsoleCli<'a>,
    update_user_on_exit: bool
) -> Result<ChannelManagerPlainTextWallet> {
    if let Some(channel_starts_with) = cli.matches.value_of(cli.arg_keys.channel_starts_with) {
        get_channel_manager_for_channel_starts_with(channel_starts_with, user_store, options, update_user_on_exit).await
    } else {
        bail!("[Management Console] You need to specify CLI argument '--{}'", cli.arg_keys.channel_starts_with);
    }
}

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

async fn println_channel_status<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>)
{
    let mut channel_exists = false;
    if let Some(author) = &channel_manager.author {
        let starts_with = cli.matches.value_of(cli.arg_keys.channel_starts_with).expect(
            format!("Error on fetching value from cli.matches for {}", cli.arg_keys.channel_starts_with).as_str()
        );
        match author.announcement_link() {
            Some(link) => {
                println_announcement_link(
                    link,
                    format!("Channel details for channel ID starting with '{}'", starts_with).as_str()
                );
                channel_exists = true
            },
            _ => {},
        }
    }
    if !channel_exists {
        println!("[Management Console] No existing channel found.");
    }
}


async fn init_sensor<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>) -> Result<()> {
    println!("[Management Console] Initializing remote sensor");
    let announcement_link = create_channel(channel_manager).await?;

    let mut remote_manager_options: Option<RemoteSensorOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_manager_options = Some(RemoteSensorOptions {
            http_url: iota_bridge_url,
            confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC,
        });
    }
    let remote_manager = RemoteSensor::new(remote_manager_options);
    println!("[Management Console] Using {} as iota-bridge url", remote_manager.get_proxy_url());

    println!("[Management Console] Sending subscribe_announcement_link command to remote sensor.\n");
    let subscription_confirm = remote_manager.subscribe_to_channel(announcement_link.to_string().as_str()).await?;

    if subscription_confirm.initialization_cnt == INITIALIZATION_CNT_MAX_VALUE {
        println_maximum_initialization_cnt_reached_warning("SManagement Console", subscription_confirm.initialization_cnt);
    }

    println!("
[Management Console] Received confirmation for successful Subscription from remote sensor.
                     Initialization count is {}
                     Creating keyload_message for
                            subscription: {}
                            public key: {}\n",
             subscription_confirm.initialization_cnt,
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

const DB_FILE_PATH_AND_NAME: &str = "user-states-management-console.sqlite3";


#[tokio::main]
async fn main() -> Result<()> {

    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = ManagementConsoleCli::new(&matches_and_options, &ARG_KEYS) ;

    println!("[Management Console] Using node '{}' for tangle connection", cli.node);


    let db_connection_opt = DbFileBasedDaoManagerOptions {
        file_path_and_name: DB_FILE_PATH_AND_NAME.to_string()
    };
    let mut user_store = UserDataStore::new(db_connection_opt);

    let mut print_usage_help = false;
    if let Some(mut channel_manager) = create_channel_manager(&mut user_store, &cli).await {
        if cli.matches.is_present(cli.arg_keys.create_channel) {
            create_channel(&mut channel_manager).await?;
        } else if cli.matches.is_present(cli.arg_keys.subscription_link) {
            send_keyload_message_cli(&mut channel_manager, &cli).await?;
        } else if cli.matches.is_present(cli.arg_keys.init_sensor) {
            init_sensor(&mut channel_manager, &cli).await?;
        } else if cli.matches.is_present(cli.arg_keys.println_channel_status) {
            println_channel_status(&mut channel_manager, &cli ).await;
        } else {
            print_usage_help = true;
        }
    } else if cli.matches.is_present(cli.arg_keys.run_explorer_api_server) {
        let message_explorer_listener_address = cli.matches.value_of(cli.arg_keys.run_explorer_api_server).unwrap();
        run_explorer_api_server(
            user_store,
            ExplorerOptions {
                iota_node_url: cli.node.to_string(),
                wallet_filename: get_management_console_wallet_filename(&cli)?,
                db_file_name: DB_FILE_PATH_AND_NAME.to_string(),
                listener_ip_address_port: message_explorer_listener_address.to_string(),
                streams_user_serialization_password: SUSEE_CONST_SECRET_PASSWORD.to_string()
            }
        ).await?;
    } else {
        println!("[Management Console] Error: Could not create channel_manager");
        print_usage_help = true;
    }

    if print_usage_help {
        println!("[Management Console] You need to specify one of these options: --{}, --{}, --{}, --{} or --{}\n",
                 cli.arg_keys.create_channel,
                 cli.arg_keys.subscription_link,
                 cli.arg_keys.init_sensor,
                 cli.arg_keys.run_explorer_api_server,
                 cli.arg_keys.println_channel_status,
        );
    }

    Ok(())
}

