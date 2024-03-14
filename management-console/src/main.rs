use core::str::FromStr;

use anyhow::{
    Result,
    bail,
    anyhow
};

use streams::{
    Address,
};
use streams::id::{
    PermissionDuration,
    Permissioned
};

use streams_tools::{
    channel_manager::SubscriberData,
    subscriber_manager::println_maximum_initialization_cnt_reached_warning,
    ChannelManagerPlainTextWallet,
    UserDataStore,
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
    binary_persist::{
        INITIALIZATION_CNT_MAX_VALUE,
        DevEuiHandshake,
        KeyloadRegistration,
        Subscription,
    },
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
    get_wallet_filename,
    assert_data_dir_existence,
    get_data_folder_file_path,
    set_env_rust_log_variable_if_not_defined_by_env,
};

use cli::{
    ARG_KEYS,
    get_arg_matches,
    ManagementConsoleCli,
};

use crate::multiple_sensor_init::init_sensor_in_own_thread;

mod cli;
mod multiple_sensor_init;

fn get_management_console_wallet_filename<'a>(cli: &ManagementConsoleCli<'a>) -> Result<String> {
    get_wallet_filename(
        &cli.matches,
        cli.arg_keys.base.wallet_file,
        &cli.data_dir,
        "wallet-management-console.txt",
    )
}

fn get_multi_channel_manager_options<'a>(cli: &ManagementConsoleCli<'a>) -> Result<MultiChannelManagerOptions> {
    let wallet_filename= get_management_console_wallet_filename(cli)?;
    Ok(MultiChannelManagerOptions{
        iota_node: cli.node.to_string(),
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
    log::info!(
        "[Management Console] {}:
                     Announcement Link: {}
                          Tangle Index: {:#?}\n",
        comment,
        link.to_string(),
        hex::encode(link.to_msg_index())
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
    if let Some(author) = &channel_manager.user {
        let starts_with = cli.matches.value_of(cli.arg_keys.channel_starts_with).expect(
            format!("Error on fetching value from cli.matches for {}", cli.arg_keys.channel_starts_with).as_str()
        );
        match author.stream_address() {
            Some(link) => {
                println_announcement_link(
                    &link,
                    format!("Channel details for channel ID starting with '{}'", starts_with).as_str()
                );
                channel_exists = true
            },
            _ => {},
        }
    }
    if !channel_exists {
        log::info!("No existing channel found.");
    }
}


async fn init_sensor<'a> (user_store: &UserDataStore, cli: &ManagementConsoleCli<'a>, options: &MultiChannelManagerOptions) -> Result<()> {
    log::info!("Initializing remote sensor");
    let remote_sensor = RemoteSensor::new(Some(create_remote_sensor_options(cli, None)));
    let dev_eui_handshake = perform_dev_eui_handshake(&remote_sensor).await?;
    remote_sensor.set_dev_eui(dev_eui_handshake.dev_eui.as_str());
    let mut channel_manager  = get_initial_channel_manager(
        user_store,
        &options,
        Some(dev_eui_handshake.dev_eui.clone())
    ).await?;
    let announcement_link = create_channel(&mut channel_manager).await?;
    let subscription = subscribe_remote_sensor_to_channel(&remote_sensor, announcement_link).await?;
    let _keyload_registration = make_remote_sensor_register_keyload_msg(&mut channel_manager, &remote_sensor, subscription).await?;
    Ok(())
}

async fn init_multiple_sensors<'a>(user_store: &UserDataStore, cli: &ManagementConsoleCli<'a>) -> Result<()> {
    log::info!("Initializing multiple remote sensors");
    loop {
        let dev_eui = {
            let remote_sensor = RemoteSensor::new(Some(create_remote_sensor_options(cli, None)));
            let dev_eui_handshake = perform_dev_eui_handshake(&remote_sensor).await?;
            dev_eui_handshake.dev_eui.clone()
        };

        match init_sensor_in_own_thread(user_store, cli, dev_eui).await{
            Ok(_) => {}
            Err(_) => {
                break;
            }
        };
    }
    Ok(())
}

async fn perform_dev_eui_handshake<'a>(remote_sensor: &RemoteSensor) -> Result<DevEuiHandshake> {
    log::info!("Using {} as iota-bridge url", remote_sensor.get_proxy_url());

    log::info!("DevEUI: {} - Sending dev_eui_handshake command to remote sensor.", remote_sensor.get_dev_eui_command());
    let dev_eui_handshake = remote_sensor.dev_eui_handshake().await?;

    log::info!("DevEUI: {} - Received dev_eui_handshake from remote sensor.", dev_eui_handshake.dev_eui);
    Ok(dev_eui_handshake)
}

async fn subscribe_remote_sensor_to_channel(remote_sensor: &RemoteSensor, announcement_link: Address) -> Result<Subscription> {
    log::info!("DevEUI: {} - Sending subscribe_announcement_link command to remote sensor.", remote_sensor.get_dev_eui_command());
    let subscription_confirm = remote_sensor.subscribe_to_channel(announcement_link.to_string().as_str()).await?;

    if subscription_confirm.initialization_cnt == INITIALIZATION_CNT_MAX_VALUE {
        println_maximum_initialization_cnt_reached_warning("SManagement Console", subscription_confirm.initialization_cnt);
    }

    log::info!("
DevEUI: {} - Received confirmation for successful Subscription from remote sensor.
                     Initialization count is {}
                     Creating keyload_message for
                            subscription: {}
                            public key: {}\n",
             remote_sensor.get_dev_eui_confirm(),
             subscription_confirm.initialization_cnt,
             subscription_confirm.subscription_link,
             subscription_confirm.pup_key,
    );
    Ok(subscription_confirm)
}

async fn make_remote_sensor_register_keyload_msg(
    channel_manager: &mut ChannelManagerPlainTextWallet,
    remote_sensor: &RemoteSensor,
    subscription: Subscription
) -> Result<KeyloadRegistration> {
    let keyload_msg_link = send_keyload_message(
        channel_manager,
        subscription.subscription_link.as_str(),
        subscription.pup_key.as_str()
    ).await?;

    log::info!("DevEUI: {} - Sending register_keyload_msg command to remote sensor.", remote_sensor.get_dev_eui_command());
    let keyload_registration = remote_sensor.register_keyload_msg(keyload_msg_link.to_string().as_str()).await?;
    log::info!("
DevEUI: {0} - Received confirmation for successful KeyloadRegistration from remote sensor.
                     =========> Sensor {0} has been fully initialized <==========="
        , remote_sensor.get_dev_eui_confirm()
    );
    Ok(keyload_registration)
}

fn create_remote_sensor_options<'a>(cli: &ManagementConsoleCli<'a>, opt_dev_eu: Option<String>) -> RemoteSensorOptions {
    let mut remote_options = RemoteSensorOptions::default();
    remote_options.confirm_fetch_wait_sec = SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC;

    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_options.http_url = iota_bridge_url.to_string();
    }

    if let Some(dev_eui) = cli.matches.value_of(cli.arg_keys.dev_eui) {
        remote_options.dev_eui = dev_eui.to_string();
    } else {
        if let Some(dev_eui) = opt_dev_eu {
            remote_options.dev_eui = dev_eui;
        }
    }
    remote_options
}

async fn send_keyload_message_cli<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, cli: &ManagementConsoleCli<'a>) -> Result<Address>
{
    let sub_msg_link_string = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
    let pub_key_str = cli.matches.value_of(cli.arg_keys.subscription_pub_key).unwrap();
    send_keyload_message(channel_manager, sub_msg_link_string, pub_key_str).await
}

async fn send_keyload_message<'a> (channel_manager: &mut ChannelManagerPlainTextWallet, sub_msg_link_string: &str, pub_key_str: &str) -> Result<Address>
{
    let subscription_msg_link = Address::from_str(sub_msg_link_string).map_err(|e|anyhow!(e))?;
    let pub_key = hex::decode(pub_key_str).unwrap();
    let keyload_msg_link = channel_manager.add_subscribers(&vec![ SubscriberData {
        subscription_link: & subscription_msg_link,
        permissioned_public_key: Permissioned::ReadWrite(pub_key.as_slice(), PermissionDuration::Perpetual)
    }]).await?;

    log::info!(
        "\
A keyload message has been created with the following keyload link:
                     Keyload link: {}
                     Tangle Index: {:#?}
", keyload_msg_link.to_string(), hex::encode(keyload_msg_link.to_msg_index())
    );

    Ok(keyload_msg_link)
}

const DB_FILE_PATH_AND_NAME: &str = "user-states-management-console.sqlite3";


#[tokio::main]
async fn main() -> Result<()> {
    set_env_rust_log_variable_if_not_defined_by_env("info");
    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = ManagementConsoleCli::new(&matches_and_options, &ARG_KEYS) ;
    assert_data_dir_existence(&cli.data_dir)?;

    log::info!("Using node '{}' for tangle connection", cli.node);


    let db_connection_opt = DbFileBasedDaoManagerOptions {
        file_path_and_name: get_data_folder_file_path(&cli.data_dir, DB_FILE_PATH_AND_NAME)
    };
    let mut user_store = UserDataStore::new(db_connection_opt);

    let mut print_usage_help = false;

    let options = get_multi_channel_manager_options(&cli)
        .expect("Could not create multi_channel_manager_options");

    if cli.matches.is_present(cli.arg_keys.create_channel) {
        let mut channel_manager = get_initial_channel_manager(&user_store, &options, None).await.unwrap();
        create_channel(&mut channel_manager).await?;
    }
    else if cli.matches.is_present(cli.arg_keys.subscription_link) {
        let sub_msg_link_str = cli.matches.value_of(cli.arg_keys.subscription_link).unwrap();
        if let Some(channel_id ) = get_channel_id_from_link(sub_msg_link_str) {
            let mut channel_manager = get_channel_manager_for_channel_id(channel_id.as_str(), &user_store, &options).await.unwrap();
            send_keyload_message_cli(&mut channel_manager, &cli).await?;
        } else {
            log::error!("Could not parse channel_id from CLI argument '--{}'. Argument value is {}",
                        cli.arg_keys.subscription_link,
                        sub_msg_link_str,
            )
        }
    } else if cli.matches.is_present(cli.arg_keys.println_channel_status) {
        let mut channel_manager = get_channel_manager_for_cli_arg_channel_starts_with(&mut user_store, &options, &cli, false).await.unwrap();
        println_channel_status(&mut channel_manager, &cli ).await;
    }
    else if cli.matches.is_present(cli.arg_keys.init_sensor) {
        init_sensor(&user_store, &cli, &options).await?;
    }
    else if cli.matches.is_present(cli.arg_keys.init_multiple_sensors) {
        init_multiple_sensors(&mut user_store, &cli).await?;
    }
    else if cli.matches.is_present(cli.arg_keys.run_explorer_api_server) {
        let message_explorer_listener_address = cli.matches.value_of(cli.arg_keys.run_explorer_api_server).unwrap();
        run_explorer_api_server(
            user_store,
            ExplorerOptions {
                iota_node: cli.node.to_string(),
                wallet_filename: get_management_console_wallet_filename(&cli)?,
                db_file_name: get_data_folder_file_path(&cli.data_dir, DB_FILE_PATH_AND_NAME),
                listener_ip_address_port: message_explorer_listener_address.to_string(),
                streams_user_serialization_password: SUSEE_CONST_SECRET_PASSWORD.to_string()
            }
        ).await?;
    } else {
        log::error!("Error: None of expected CLI Arguments founds");
        print_usage_help = true;
    }

    if print_usage_help {
        println!("[Management Console] You need to specify one of these options: --{}, --{}, --{}, --{}, --{} or --{}\n",
                 cli.arg_keys.create_channel,
                 cli.arg_keys.subscription_link,
                 cli.arg_keys.init_sensor,
                 cli.arg_keys.init_multiple_sensors,
                 cli.arg_keys.run_explorer_api_server,
                 cli.arg_keys.println_channel_status,
        );
    }

    Ok(())
}

