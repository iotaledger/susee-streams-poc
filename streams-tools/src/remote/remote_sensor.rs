use crate::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    http::http_protocol_command::RequestBuilderCommand,
    http::http_protocol_confirm::RequestBuilderConfirm,
    binary_persist::{
        Confirmation,
        BinaryPersist,
        EnumeratedPersistableArgs,
        Subscription,
        SubscriberStatus,
        KeyloadRegistration,
        ClearClientState,
        SendMessages
    }
};

use hyper::{
    Client,
    body,
    Body,
    client::HttpConnector,
    http::StatusCode,
};

use anyhow::{
    Result,
    bail,
};

use std::{
    fmt,
    fmt::Display,
    time::Duration,
    thread,
    io::{
        stdout,
        Write
    }
};

use log;

type HttpClient = Client<HttpConnector, Body>;

pub struct RemoteSensorOptions<'a> {
    pub http_url: &'a str,
    pub confirm_fetch_wait_sec: u32,
}

impl Default for RemoteSensorOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
            confirm_fetch_wait_sec: 5,
        }
    }
}

impl fmt::Display for RemoteSensorOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RemoteSensorOptions: http_url: {}", self.http_url)
    }
}

pub struct RemoteSensor<'a> {
    options: RemoteSensorOptions<'a>,
    http_client: HttpClient,
}

impl<'a> RemoteSensor<'a> {

    pub fn new(options: Option<RemoteSensorOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[RemoteSensor.new()] Initializing instance with options:\n       {}\n", options);
        Self {
            options,
            http_client: HttpClient::new(),
        }
    }

    pub fn get_proxy_url(&self) -> &'a str { self.options.http_url }

    fn get_request_builder_command(&self) -> RequestBuilderCommand {
        RequestBuilderCommand::new(self.get_proxy_url())
    }

    fn get_request_builder_confirm(&self) -> RequestBuilderConfirm {
        RequestBuilderConfirm::new(self.get_proxy_url())
    }

    async fn poll_confirmation<T>(&self) -> Result<T>
        where
            T: EnumeratedPersistableArgs<Confirmation> + BinaryPersist + Display
    {
        let confirm_fetch_wait_sec = self.options.confirm_fetch_wait_sec;
        loop {
            for s in 0..confirm_fetch_wait_sec {
                print!("Fetching next confirmation in {} secs\r", confirm_fetch_wait_sec - s);
                stdout().flush().unwrap();
                thread::sleep(Duration::from_secs(1));
            }

            if let Ok((confirmation, buffer)) = self.fetch_next_confirmation().await {
                if confirmation != Confirmation::NO_CONFIRMATION {
                    return self.process_confirmation::<T>(confirmation, buffer).await;
                } else {
                    println!("Received Confirmation::NO_CONFIRMATION    ");
                }
            } else {
                log::error!("[fn poll_confirmation] fn call fetch_next_confirmation() failed.");
            }
        }
    }

    async fn process_confirmation<T: >(&self, confirm: Confirmation, buffer: Vec<u8>) -> Result<T>
        where
            T: EnumeratedPersistableArgs<Confirmation> + BinaryPersist + Display
    {
        if &confirm == T::INSTANCE {
            let confirmation_args = <T as BinaryPersist>::try_from_bytes(buffer.as_slice())?;
            log::info!("[fn process_confirmation] processing confirmation: {}", confirmation_args);
            Ok(confirmation_args)
        } else {
            bail!("Received confirmation does not match the expected confirmation type")
        }
    }

    async fn fetch_next_confirmation(&self) -> Result<(Confirmation, Vec<u8>)> {
        let response = self.http_client.request(
            self.get_request_builder_confirm().fetch_next_confirmation()?
        ).await?;

        if response.status() == StatusCode::OK {
            log::debug!("[RemoteSensor.fetch_next_confirmation] StatusCode::OK - process confirmation");
            let bytes = body::to_bytes(response.into_body()).await?;
            let confirmation = <Confirmation as BinaryPersist>::try_from_bytes(&bytes)?;
            Ok((confirmation, bytes.to_vec()))
        } else {
            log::error!("[RemoteSensor.fetch_next_confirmation] HTTP Error. Status: {}", response.status());
            Ok((Confirmation::NO_CONFIRMATION, Vec::<u8>::default()))
        }
    }

    pub async fn subscribe_to_channel(&self, announcement_link_str: &str) -> Result<Subscription> {
        self.http_client.request(
            self.get_request_builder_command().subscribe_to_announcement(announcement_link_str)?
        ).await?;
        self.poll_confirmation::<Subscription>().await
    }

    pub async fn register_keyload_msg(&self, keyload_msg_link_str: &str) -> Result<KeyloadRegistration> {
        self.http_client.request(
            self.get_request_builder_command().register_keyload_msg(keyload_msg_link_str)?
        ).await?;
        self.poll_confirmation::<KeyloadRegistration>().await
    }

    pub async fn send_messages(&self, file_to_send: &str) -> Result<SendMessages> {
        self.http_client.request(
            self.get_request_builder_command().send_message(file_to_send)?
        ).await?;
        self.poll_confirmation::<SendMessages>().await
    }

    pub async fn println_subscriber_status(&self) -> Result<SubscriberStatus> {
        self.http_client.request(
            self.get_request_builder_command().println_subscriber_status()?
        ).await?;

        self.poll_confirmation::<SubscriberStatus>().await
    }

    pub async fn clear_client_state(&self)  -> Result<ClearClientState> {
        self.http_client.request(
            self.get_request_builder_command().clear_client_state()?
        ).await?;
        self.poll_confirmation::<ClearClientState>().await
    }
}