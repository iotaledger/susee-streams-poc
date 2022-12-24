use super::cli::{
    SensorCli,
    ARG_KEYS,
    get_arg_matches,
};

use anyhow::{
    Result,
    anyhow,
};

use crate::{
    std::{
        ClientType,
        SubscriberManagerPlainTextWalletHttpClient,
        sensor_manager::SensorManager,
        command_fetcher::{
            CommandFetcher,
            CommandFetcherOptions
        }
    }
};

use streams_tools::{
    PlainTextWallet,
    http_client::HttpClientOptions,
    http::http_protocol_confirm::RequestBuilderConfirm,
    binary_persist::Command,
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    remote::{
        remote_sensor::{
            RemoteSensor,
            RemoteSensorOptions,
        },
        command_processor::{
            CommandProcessor,
            SensorFunctions,
            process_sensor_commands,
            run_command_fetch_loop,
            CommandFetchLoopOptions,
        }
    },
};

use susee_tools::{
    SUSEE_CONST_SECRET_PASSWORD,
    get_wallet_filename,
    SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
};

use hyper::{
    Body,
    http::Request,
};

use iota_streams::core::async_trait;
use rand::Rng;
use std::cell::Cell;

fn get_wallet(cli: &SensorCli) -> Result<PlainTextWallet> {
    let wallet_filename= get_wallet_filename(
        &cli.matches,
        cli.arg_keys.base.wallet_file,
        "wallet-sensor.txt",
    )?;

    Ok(PlainTextWallet::new(
        SUSEE_CONST_SECRET_PASSWORD,
        Some(wallet_filename.as_str()),
        None,
    ))
}

pub fn manage_mocked_lorawan_dev_eui(wallet: &mut PlainTextWallet) {
    if wallet.persist.misc_other_data.len() == 0 {
        let new_dev_eui = rand::thread_rng().gen_range(0, u64::MAX);
        wallet.persist.misc_other_data = new_dev_eui.to_string();
        wallet.write_wallet_file();
    }
    log::debug!("[Sensor - fn manage_mocked_lorawan_dev_eui()] Mocked LoRaWAN DevEUI is {}", wallet.persist.misc_other_data);
}

pub async fn create_subscriber_manager<'a>(cli: &SensorCli<'a>) -> Result<SubscriberManagerPlainTextWalletHttpClient> {
    let mut wallet = get_wallet(&cli)?;
    let mut http_client_options: HttpClientOptions = HttpClientOptions::default();
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        http_client_options.http_url = iota_bridge_url;
    }
    manage_mocked_lorawan_dev_eui(&mut wallet);
    if wallet.persist.misc_other_data.len() > 0 {
        http_client_options.dev_eui = Some(wallet.persist.misc_other_data.clone());
    }
    if cli.matches.is_present(cli.arg_keys.use_lorawan_rest_api) {
        http_client_options.use_lorawan_rest = true;
    }
    let client = ClientType::new(Some(http_client_options));
    Ok(SubscriberManagerPlainTextWalletHttpClient::new(
        client,
        wallet,
        Some(String::from("user-state-sensor.bin")),
    ).await)
}

pub async fn process_local_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {

    let mut subscriber= create_subscriber_manager(&cli).await?;
    let mut show_subscriber_state = true;

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        show_subscriber_state = false;
        SensorManager::subscribe_to_channel(announcement_link_str, &mut subscriber).await?;
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        SensorManager::send_messages(files_to_send, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        show_subscriber_state = false;
        SensorManager::register_keyload_msg(keyload_msg_link_str, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        show_subscriber_state = false;
        SensorManager::clear_client_state(&mut subscriber).await?
    }

    if show_subscriber_state {
        SensorManager::println_subscriber_status(&subscriber)?;
    }

    Ok(())
}

pub async fn process_act_as_remote_control<'a>(cli: SensorCli<'a>) -> Result<()> {
    let mut show_subscriber_state = cli.matches.is_present(cli.arg_keys.println_subscriber_status);

    let mut remote_manager_options: Option<RemoteSensorOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_manager_options = Some(RemoteSensorOptions {
            http_url: iota_bridge_url,
            confirm_fetch_wait_sec: 5,
        });
    }

    let remote_manager = RemoteSensor::new(remote_manager_options);

    println!("[Sensor] Acting as remote sensor using {} as iota-bridge url", remote_manager.get_proxy_url());

    if cli.matches.is_present(cli.arg_keys.subscribe_announcement_link) {
        let announcement_link_str = cli.matches.value_of(cli.arg_keys.subscribe_announcement_link).unwrap().trim();
        println!("[Sensor] Sending subscribe_announcement_link command to remote sensor. announcement_link: {}", announcement_link_str);
        show_subscriber_state = false;
        let confirm = remote_manager.subscribe_to_channel(announcement_link_str).await?;
        println!("[Sensor] Remote sensor confirmed Subscription: {}", confirm);
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let mut files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        if let Some(first_file) = files_to_send.nth(0) {
            println!("[Sensor] Sending files_to_send command to remote sensor.");
            let confirm = remote_manager.send_messages(first_file).await?;
            println!("[Sensor] Remote sensor confirmed files_to_send: {}", confirm);
        } else {
            println!("[Sensor] WARNING: Could not find any filename in files_to_send list.");
        }
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli.matches.value_of(cli.arg_keys.register_keyload_msg).unwrap().trim();
        println!("[Sensor] Sending register_keyload_msg command to remote sensor. keyload_msg_link: {}", keyload_msg_link_str);
        show_subscriber_state = false;
        let confirm = remote_manager.register_keyload_msg(keyload_msg_link_str).await?;
        println!("[Sensor] Remote sensor confirmed KeyloadRegistration: {}", confirm);
    }

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        println!("[Sensor] Sending clear_client_state command to remote sensor.");
        let confirm = remote_manager.clear_client_state().await?;
        println!("[Sensor] Remote sensor confirmed ClearClientState: {}", confirm);
    }

    if show_subscriber_state {
        let confirm = remote_manager.println_subscriber_status().await?;
        println!("[Sensor] Remote sensor SubscriberStatus: {}", confirm);
    }

    Ok(())
}


struct CmdProcessor<'a> {
    command_fetcher: CommandFetcher,
    iota_bridge_url: String,
    cli: SensorCli<'a>,
    initialization_has_been_completed: Cell<bool>,
}

impl<'a> CmdProcessor<'a> {
    pub fn new(iota_bridge_url: &str, cli: SensorCli<'a>) -> CmdProcessor<'a> {
        CmdProcessor {
            command_fetcher: CommandFetcher::new(
                Some(CommandFetcherOptions{ http_url: String::from(iota_bridge_url) }),
            ),
            iota_bridge_url: String::from(iota_bridge_url),
            cli,
            initialization_has_been_completed: Cell::new(false),
        }
    }
}

#[async_trait(?Send)]
impl<'a> SensorFunctions for CmdProcessor<'a> {
    type SubscriberManager = SubscriberManagerPlainTextWalletHttpClient;

    fn get_iota_bridge_url(&self) -> &str {
        self.iota_bridge_url.as_str()
    }

    async fn subscribe_to_channel(
        &self, announcement_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let (sub_msg_link, public_key_str) = SensorManager::subscribe_to_channel(announcement_link_str, subscriber_mngr).await
            .expect("Error on calling SensorManager::subscribe_to_channel");
        confirm_req_builder.subscription(sub_msg_link, public_key_str)
    }

    async fn send_content_as_msg(
        &self, message_key: String, subscriber: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let prev_message = SensorManager::send_file_content_as_msg(message_key.as_str(), subscriber).await
            .expect("Error on calling SensorManager::send_file_content_as_msg");
        confirm_req_builder.send_message(prev_message.to_string())
    }

    async fn register_keyload_msg(
        &self, keyload_msg_link_str: &str, subscriber_mngr: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        SensorManager::register_keyload_msg(keyload_msg_link_str, subscriber_mngr).await
            .expect("Error on calling SensorManager::register_keyload_msg");
        self.initialization_has_been_completed.set(true);
        confirm_req_builder.keyload_registration()
    }

    fn println_subscriber_status<'b>(
        &self, subscriber_manager: &Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        let (previous_message_link, subs) = SensorManager::println_subscriber_status(subscriber_manager)
            .expect("Error on calling SensorManager::println_subscriber_status");
        confirm_req_builder.subscriber_status(previous_message_link, subs)
    }

    async fn clear_client_state<'b>(
        &self, subscriber_manager: &mut Self::SubscriberManager, confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>
    {
        SensorManager::clear_client_state(subscriber_manager).await
            .expect("Error on calling SensorManager::clear_client_state");
        confirm_req_builder.clear_client_state()
    }
}

#[async_trait(?Send)]
impl<'a> CommandProcessor for CmdProcessor<'a> {
    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)> {
        if !self.cli.matches.is_present(self.cli.arg_keys.exit_after_successful_initialization)
            || !self.initialization_has_been_completed.get() {
            self.command_fetcher.fetch_next_command().await
        } else {
            Ok((Command::STOP_FETCHING_COMMANDS, Vec::<u8>::new()))
        }
    }

    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        self.command_fetcher.send_confirmation(confirmation_request).await
    }

    async fn process_command(&self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>> {
        println!("Received Command::{}", command);
        let mut subscriber= create_subscriber_manager(&self.cli).await?;

        let confirmation_request = process_sensor_commands(
            self, &mut subscriber, command, buffer
        ).await.expect("Error on processing sensor commands");

        confirmation_request.ok_or(anyhow!("No confirmation_request received"))
    }
}

pub async fn process_act_as_remote_controlled_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {
    let iota_bridge_url = cli.matches.value_of(cli.arg_keys.iota_bridge_url)
        .unwrap_or(STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);
    let cmd_processor = CmdProcessor::new(iota_bridge_url, cli);

    run_command_fetch_loop(
        cmd_processor,
        Some(
            CommandFetchLoopOptions{
                confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC
        })
    ).await
}

pub async fn process_main() -> Result<()> {

    let matches_and_options = get_arg_matches();
    let cli = SensorCli::new(&matches_and_options, &ARG_KEYS) ;

    if cli.matches.is_present(cli.arg_keys.act_as_remote_control) {
        process_act_as_remote_control(cli).await
    } else if cli.matches.is_present(cli.arg_keys.act_as_remote_controlled_sensor) {
        process_act_as_remote_controlled_sensor(cli).await
    } else {
        process_local_sensor(cli).await
    }
}
