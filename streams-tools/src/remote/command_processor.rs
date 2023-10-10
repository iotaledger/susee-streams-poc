use std::{
    time::Duration,
    thread,
    io::{
        stdout,
        Write
    }
};

use hyper::{
    Body,
    http::Request
};

use anyhow::Result;

use async_trait::async_trait;

use crate::{
    http::http_protocol_confirm::RequestBuilderConfirm,
    binary_persist::{
        Command,
        SubscribeToAnnouncement,
        BinaryPersist,
        StartSendingMessages,
        RegisterKeyloadMessage
    },
};

pub struct CommandFetchLoopOptions {
    pub confirm_fetch_wait_sec: u32,
}

impl Default for CommandFetchLoopOptions {
    fn default() -> Self {
        Self {
            confirm_fetch_wait_sec: 5,
        }
    }
}


#[async_trait(?Send)]
pub trait CommandProcessor {
    async fn fetch_next_command(&self) -> Result<(Command, Vec<u8>)>;
    async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()>;
    async fn process_command(&self, command: Command, buffer: Vec<u8>) -> Result<Request<Body>>;
}

pub async fn run_command_fetch_loop(command_processor: impl CommandProcessor, options: Option<CommandFetchLoopOptions>) -> Result<()> {
    let opt = options.unwrap_or_default();
    loop {
        if let Ok((command, buffer)) = command_processor.fetch_next_command().await {
            match command {
                Command::NO_COMMAND => {
                    log::info!("[fn run_command_fetch_loop()]Received Command::NO_COMMAND    ");
                },
                Command::STOP_FETCHING_COMMANDS => {
                    log::info!("[fn run_command_fetch_loop()]Received Command::STOP_FETCHING_COMMANDS - Will exit command fetch loop.");
                    return Ok(());
                },
                _ => {
                    log::info!("[fn run_command_fetch_loop()] Starting process_command for command: {}.", command);
                    match command_processor.process_command(command, buffer).await {
                        Ok(confirmation_request) => {
                            // TODO: Retries in case of errors could be useful
                            log::debug!("[fn run_command_fetch_loop()] Calling command_processor.send_confirmation for confirmation_request");
                            command_processor.send_confirmation(confirmation_request).await?;
                        },
                        Err(err) => {
                            log::error!("[fn run_command_fetch_loop()] process_command() returned error: {}", err);
                        }
                    };
                }
            }
        } else {
            log::error!("[fn run_command_fetch_loop()] command_processor.fetch_next_command() failed.");
        }

        for s in 0..opt.confirm_fetch_wait_sec {
            print!("Fetching next command in {} secs\r", opt.confirm_fetch_wait_sec - s);
            stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[async_trait(?Send)]
pub trait SensorFunctions {
    type SubscriberManager;

    fn get_iota_bridge_url(&self) -> String;

    async fn subscribe_to_channel(
        &self,
        announcement_link_str: &str,
        subscriber_mngr: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>;

    async fn send_content_as_msg(
        &self,
        message_key: String,
        subscriber: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>;

    async fn register_keyload_msg(
        &self,
        keyload_msg_link_str: &str,
        subscriber_mngr: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>;

    fn println_subscriber_status<'a> (
        &self,
        subscriber_manager: &Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>;

    async fn clear_client_state<'a> (
        &self,
        subscriber_manager: &mut Self::SubscriberManager,
        confirm_req_builder: &RequestBuilderConfirm
    ) -> hyper::http::Result<Request<Body>>;
}

pub async fn process_sensor_commands<SensorT: SensorFunctions>(
    sensor: &SensorT, subscriber: &mut SensorT::SubscriberManager, command: Command, buffer: Vec<u8>
) -> Result<Option<Request<Body>>>
{
    let confirm_req_builder = RequestBuilderConfirm::new(sensor.get_iota_bridge_url().as_str());
    let mut confirmation_request: Option<Request<Body>> = None;

    if command == Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK {
        let cmd_args = SubscribeToAnnouncement::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_sensor_commands()] Processing SUBSCRIBE_ANNOUNCEMENT_LINK: {}", cmd_args.announcement_link);
        confirmation_request = Some(
            sensor.subscribe_to_channel(cmd_args.announcement_link.as_str(), subscriber, &confirm_req_builder).await?
        );
    }

    if command == Command::START_SENDING_MESSAGES {
        let cmd_args = StartSendingMessages::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_sensor_commands()] Processing START_SENDING_MESSAGES: {}", cmd_args.message_template_key);
        confirmation_request = Some(
            sensor.send_content_as_msg(cmd_args.message_template_key, subscriber, &confirm_req_builder).await?
        );
    }

    if command == Command::REGISTER_KEYLOAD_MESSAGE {
        let cmd_args = RegisterKeyloadMessage::try_from_bytes(buffer.as_slice())?;
        log::info!("[fn process_sensor_commands()] Processing REGISTER_KEYLOAD_MESSAGE: {}", cmd_args.keyload_msg_link);
        confirmation_request = Some(
            sensor.register_keyload_msg(cmd_args.keyload_msg_link.as_str(), subscriber, &confirm_req_builder ).await?
        );
    }

    if command == Command::PRINTLN_SUBSCRIBER_STATUS {
        log::info!("[fn process_sensor_commands()] PRINTLN_SUBSCRIBER_STATUS");
        confirmation_request = Some(
            sensor.println_subscriber_status(subscriber, &confirm_req_builder)?
        );
    }

    if command == Command::CLEAR_CLIENT_STATE {
        log::info!("[fn process_sensor_commands()] =========> processing CLEAR_CLIENT_STATE <=========");

        confirmation_request = Some(
            sensor.clear_client_state(subscriber, &confirm_req_builder).await?
        );
    }

    Ok(confirmation_request)
}
