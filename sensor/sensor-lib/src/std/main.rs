use std::cell::Cell;
use std::str::FromStr;

use rand::Rng;

use log;

use hyper::{
    Body,
    http::Request,
};

use async_trait::async_trait;

use anyhow::{
    Result,
    anyhow,
};

use streams_tools::{
    binary_persist::Command,
    http::http_protocol_confirm::RequestBuilderConfirm,
    remote::{
        command_processor::{
            process_sensor_commands,
            run_command_fetch_loop,
            CommandFetchLoopOptions,
            CommandProcessor,
            SensorFunctions,
        },
        remote_sensor::{
            RemoteSensor,
            RemoteSensorOptions
        },
    },
    streams_transport_socket::StreamsTransportSocketOptions,
    PlainTextWallet,
    StreamsTransport,
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED,
};

use susee_tools::{
    get_wallet_filename,
    SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC,
    SUSEE_CONST_SECRET_PASSWORD,
};

use super::cli::{
    SensorCli,
    ARG_KEYS,
    get_arg_matches,
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

const MAX_SIZE_RANDOM_MESSAGE: usize = 4096;

fn get_wallet(cli: &SensorCli) -> Result<PlainTextWallet> {
    let wallet_filename = get_wallet_filename(
        &cli.matches,
        cli.arg_keys.base.wallet_file,
        &cli.data_dir,
        "wallet-sensor.txt",
    )?;

    Ok(PlainTextWallet::new(
        SUSEE_CONST_SECRET_PASSWORD,
        Some(wallet_filename.as_str()),
        None,
    ))
}

pub fn manage_mocked_lorawan_dev_eui<'a>(cli: &SensorCli<'a>, wallet: &mut PlainTextWallet) -> String {
    if wallet.persist.misc_other_data.len() == 0 {
        let new_dev_eui = if cli.matches.is_present(cli.arg_keys.dev_eui) {
            if let Some(dev_eui_str) = cli.matches.value_of(cli.arg_keys.dev_eui) {
                dev_eui_str.to_string()
            } else {
                panic!("[fn manage_mocked_lorawan_dev_eui] CLI argument {} has been used without specifying the DevEui.", cli.arg_keys.dev_eui)
            }
        } else {
            rand::thread_rng().gen_range(0, u64::MAX).to_string()
        };
        wallet.persist.misc_other_data = new_dev_eui;
        wallet.write_wallet_file();
    }
    log::debug!(
        "[Sensor - fn manage_mocked_lorawan_dev_eui()] Mocked LoRaWAN DevEUI is {}",
        wallet.persist.misc_other_data
    );
    wallet.persist.misc_other_data.clone()
}

pub async fn create_subscriber_manager<'a>(
    cli: &SensorCli<'a>,
) -> Result<SubscriberManagerPlainTextWalletHttpClient> {
    let mut wallet = get_wallet(&cli)?;
    let mut streams_transport_options: StreamsTransportSocketOptions =
        StreamsTransportSocketOptions::default();
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        streams_transport_options.http_url = iota_bridge_url.to_string();
    }
    streams_transport_options.dev_eui = Some(manage_mocked_lorawan_dev_eui(&cli, &mut wallet));
    if cli.matches.is_present(cli.arg_keys.use_lorawan_rest_api) {
        streams_transport_options.use_lorawan_rest = true;
    }
    let transport = ClientType::new(Some(streams_transport_options));
    Ok(SubscriberManagerPlainTextWalletHttpClient::new(
        transport,
        wallet,
        Some(String::from("user-state-sensor.bin")),
    )
    .await)
}

pub async fn process_local_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {
    let mut subscriber = create_subscriber_manager(&cli).await?;
    let mut show_subscriber_state = true;

    if cli
        .matches
        .is_present(cli.arg_keys.subscribe_announcement_link)
    {
        let announcement_link_str = cli
            .matches
            .value_of(cli.arg_keys.subscribe_announcement_link)
            .unwrap()
            .trim();
        show_subscriber_state = false;
        SensorManager::subscribe_to_channel(announcement_link_str, &mut subscriber).await?;
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        SensorManager::send_messages_in_endless_loop(files_to_send, &mut subscriber).await?
    }

    if cli.matches.is_present(cli.arg_keys.random_msg_of_size) {
        if let Some(msg_size_str) = cli.matches.value_of(cli.arg_keys.random_msg_of_size) {
            let msg_size = usize::from_str(msg_size_str)?;
            if msg_size > MAX_SIZE_RANDOM_MESSAGE {
                log::error!("The MSG_SIZE value needs to be a positive integer number smaller than {}", MAX_SIZE_RANDOM_MESSAGE)
            }
            SensorManager::send_random_message_in_endless_loop(msg_size, &mut subscriber).await?
        }
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli
            .matches
            .value_of(cli.arg_keys.register_keyload_msg)
            .unwrap()
            .trim();
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
    let mut show_subscriber_state = cli
        .matches
        .is_present(cli.arg_keys.println_subscriber_status);

    let mut remote_sensor_options: Option<RemoteSensorOptions> = None;
    if let Some(iota_bridge_url) = cli.matches.value_of(cli.arg_keys.iota_bridge_url) {
        remote_sensor_options = Some(RemoteSensorOptions {
            http_url: iota_bridge_url.to_string(),
            confirm_fetch_wait_sec: 5,
            dev_eui: STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED.to_string(),
        });
    }

    let remote_sensor = RemoteSensor::new(remote_sensor_options);

    log::info!(
        "[Sensor] Acting as remote sensor using {} as iota-bridge url",
        remote_sensor.get_proxy_url()
    );

    if cli
        .matches
        .is_present(cli.arg_keys.subscribe_announcement_link)
    {
        let announcement_link_str = cli
            .matches
            .value_of(cli.arg_keys.subscribe_announcement_link)
            .unwrap()
            .trim();
        log::info!("[Sensor] Sending subscribe_announcement_link command to remote sensor. announcement_link: {}", announcement_link_str);
        show_subscriber_state = false;
        let confirm = remote_sensor
            .subscribe_to_channel(announcement_link_str)
            .await?;
        log::info!("[Sensor] Remote sensor confirmed Subscription: {}", confirm);
    }

    if cli.matches.is_present(cli.arg_keys.files_to_send) {
        let mut files_to_send = cli.matches.values_of(cli.arg_keys.files_to_send).unwrap();
        if let Some(first_file) = files_to_send.nth(0) {
            log::info!("[Sensor] Sending files_to_send command to remote sensor.");
            remote_sensor.send_messages_in_endless_loop(first_file).await?;
        } else {
            log::info!("[Sensor] WARNING: Could not find any filename in files_to_send list.");
        }
    }

    if cli.matches.is_present(cli.arg_keys.register_keyload_msg) {
        let keyload_msg_link_str = cli
            .matches
            .value_of(cli.arg_keys.register_keyload_msg)
            .unwrap()
            .trim();
        log::info!(
            "[Sensor] Sending register_keyload_msg command to remote sensor. keyload_msg_link: {}",
            keyload_msg_link_str
        );
        show_subscriber_state = false;
        let confirm = remote_sensor
            .register_keyload_msg(keyload_msg_link_str)
            .await?;
        log::info!(
            "[Sensor] Remote sensor confirmed KeyloadRegistration: {}",
            confirm
        );
    }

    if cli.matches.is_present(cli.arg_keys.clear_client_state) {
        log::info!("[Sensor] Sending clear_client_state command to remote sensor.");
        let confirm = remote_sensor.clear_client_state().await?;
        log::info!(
            "[Sensor] Remote sensor confirmed ClearClientState: {}",
            confirm
        );
    }

    if show_subscriber_state {
        let confirm = remote_sensor.println_subscriber_status().await?;
        log::info!("[Sensor] Remote sensor SubscriberStatus: {}", confirm);
    }

    Ok(())
}

struct CmdProcessor<'a> {
    command_fetcher: CommandFetcher,
    iota_bridge_url: String,
    dev_eui: String,
    cli: SensorCli<'a>,
    initialization_has_been_completed: Cell<bool>,
}

impl<'a> CmdProcessor<'a> {
    pub fn new(iota_bridge_url: &str, cli: SensorCli<'a>, dev_eui: &str) -> CmdProcessor<'a> {
        CmdProcessor {
            command_fetcher: CommandFetcher::new(Some(CommandFetcherOptions {
                http_url: iota_bridge_url.to_string(),
                // We set this true here because there are no other usecases for the
                // ACT_AS_REMOTE_CONTROLLED_SENSOR mode than initializing the sensor.
                dev_eui_handshake_first: true,
                dev_eui: dev_eui.to_string(),
            })),
            iota_bridge_url: iota_bridge_url.to_string(),
            dev_eui: dev_eui.to_string(),
            cli,
            initialization_has_been_completed: Cell::new(false),
        }
    }
}

#[async_trait(?Send)]
impl<'a> SensorFunctions for CmdProcessor<'a> {
    type SubscriberManager = SubscriberManagerPlainTextWalletHttpClient;

    fn get_iota_bridge_url(&self) -> String {
        self.iota_bridge_url.clone()
    }

    fn get_dev_eui(&self) -> String {
        self.dev_eui.clone()
    }

    async fn subscribe_to_channel(
        &self,
        announcement_link_str: &str,
        subscriber_mngr: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        let (sub_msg_link, public_key_str, initialization_cnt) =
            SensorManager::subscribe_to_channel(announcement_link_str, subscriber_mngr)
                .await
                .expect("Error on calling SensorManager::subscribe_to_channel");
        confirm_req_builder.subscription(sub_msg_link, public_key_str, initialization_cnt)
    }

    async fn dev_eui_handshake(
        &self,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        confirm_req_builder.dev_eui_handshake(
            self.dev_eui.clone()
        )
    }

    async fn send_content_as_msg_in_endless_loop(
        &self,
        message_key: String,
        subscriber: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        SensorManager::send_file_content_as_msg_in_endless_loop(message_key.as_str(), subscriber)
                .await
                .expect("Error on calling SensorManager::send_file_content_as_msg_in_endless_loop");
        confirm_req_builder.send_messages_in_endless_loop()
    }


    async fn send_random_msg_in_endless_loop(
        &self,
        msg_size: usize,
        subscriber: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        SensorManager::send_random_message_in_endless_loop(msg_size, subscriber)
                .await
                .expect("Error on calling SensorManager::send_random_message_in_endless_loop");
        confirm_req_builder.send_messages_in_endless_loop()
    }

    async fn register_keyload_msg(
        &self,
        keyload_msg_link_str: &str,
        subscriber_mngr: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        SensorManager::register_keyload_msg(keyload_msg_link_str, subscriber_mngr)
            .await
            .expect("Error on calling SensorManager::register_keyload_msg");
        self.initialization_has_been_completed.set(true);
        confirm_req_builder.keyload_registration()
    }

    fn println_subscriber_status<'b>(
        &self,
        subscriber_manager: &Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        let (previous_message_link, subs) =
            SensorManager::println_subscriber_status(subscriber_manager)
            .expect("Error on calling SensorManager::println_subscriber_status");
        confirm_req_builder.subscriber_status(previous_message_link, subs)
    }

    async fn clear_client_state<'b>(
        &self,
        subscriber_manager: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm,
    ) -> hyper::http::Result<Request<Body>> {
        SensorManager::clear_client_state(subscriber_manager)
            .await
            .expect("Error on calling SensorManager::clear_client_state");
        confirm_req_builder.clear_client_state()
    }
}

#[async_trait(?Send)]
impl<'a> CommandProcessor for CmdProcessor<'a> {
    fn get_dev_eui(&self) -> String {
        self.dev_eui.clone()
    }

    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)> {
        if !self
            .cli
            .matches
            .is_present(self.cli.arg_keys.exit_after_successful_initialization)
            || !self.initialization_has_been_completed.get()
        {
            self.command_fetcher.fetch_next_command().await
        } else {
            Ok((Command::STOP_FETCHING_COMMANDS, Vec::<u8>::new()))
        }
    }

    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        self.command_fetcher
            .send_confirmation(confirmation_request)
            .await
    }

    async fn process_command(&self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>> {
        log::info!("DevEUI: {} - Received Command::{}", self.dev_eui, command);
        let mut subscriber = create_subscriber_manager(&self.cli).await?;

        let confirmation_request = process_sensor_commands(self, &mut subscriber, command, buffer)
            .await
            .expect("Error on processing sensor commands");

        confirmation_request.ok_or(anyhow!("No confirmation_request received"))
    }
}

pub async fn process_act_as_remote_controlled_sensor<'a>(cli: SensorCli<'a>) -> Result<()> {
    let iota_bridge_url = cli
        .matches
        .value_of(cli.arg_keys.iota_bridge_url)
        .unwrap_or(STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL);
    let mut wallet = get_wallet(&cli)?;
    let dev_eui = manage_mocked_lorawan_dev_eui(&cli, &mut wallet);
    let cmd_processor = CmdProcessor::new(iota_bridge_url, cli, dev_eui.as_str());

    run_command_fetch_loop(
        cmd_processor,
        Some(CommandFetchLoopOptions {
            confirm_fetch_wait_sec: SUSEE_CONST_COMMAND_CONFIRM_FETCH_WAIT_SEC,
        }),
    )
    .await
}

pub async fn process_main() -> Result<()> {
    let matches_and_options = get_arg_matches();
    let cli = SensorCli::new(&matches_and_options, &ARG_KEYS);

    if cli.matches.is_present(cli.arg_keys.act_as_remote_control) {
        process_act_as_remote_control(cli).await
    } else if cli
        .matches
        .is_present(cli.arg_keys.act_as_remote_controlled_sensor)
    {
        process_act_as_remote_controlled_sensor(cli).await
    } else {
        process_local_sensor(cli).await
    }
}
