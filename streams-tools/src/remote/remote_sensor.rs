use crate::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    STREAMS_TOOLS_CONST_ANY_DEV_EUI,
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
        DevEuiHandshake
    }
};

use hyper::{
    Client,
    body,
    Body,
    client::HttpConnector,
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

type HttpClient = Client<HttpConnector, Body>;

#[derive(Clone)]
pub struct RemoteSensorOptions {
    pub http_url: String,
    pub confirm_fetch_wait_sec: u32,
    pub dev_eui: String,
}

impl Default for RemoteSensorOptions {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL.to_string(),
            confirm_fetch_wait_sec: 5,
            dev_eui: STREAMS_TOOLS_CONST_ANY_DEV_EUI.to_string(),
        }
    }
}

impl fmt::Display for RemoteSensorOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RemoteSensorOptions:\n   http_url: {}\n   DevEUI: {}", self.http_url, self.dev_eui)
    }
}

pub struct RemoteSensor {
    options: RemoteSensorOptions,
    http_client: HttpClient,
    request_builder_command: RequestBuilderCommand,
    request_builder_confirm: RequestBuilderConfirm,
}

impl RemoteSensor {

    pub fn new(options: Option<RemoteSensorOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Initializing instance with options:\n       {}\n", options);
        Self {
            http_client: HttpClient::new(),
            request_builder_command: RequestBuilderCommand::new(
                options.http_url.as_str(),
                options.dev_eui.as_str(),
                false
            ),
            request_builder_confirm: RequestBuilderConfirm::new(
                options.http_url.as_str(),
                options.dev_eui.as_str()
            ),
            options,
        }
    }

    pub fn get_proxy_url(&self) -> &str { self.options.http_url.as_str() }
    pub fn get_dev_eui_command(&self) -> String { self.request_builder_command.get_dev_eui() }
    pub fn get_dev_eui_confirm(&self) -> String { self.request_builder_confirm.get_dev_eui() }

    pub fn set_dev_eui(&self, dev_eui: &str) {
        log::debug!("[fn set_dev_eui()] Setting dev_eui for request_builder_command and request_builder_confirm to : '{}'", dev_eui);
        self.request_builder_command.set_dev_eui(dev_eui);
        self.request_builder_confirm.set_dev_eui(dev_eui);
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
                    log::info!("DevEUI: {} - Received Confirmation::NO_CONFIRMATION", self.request_builder_confirm.get_dev_eui());
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
            log::info!("[fn process_confirmation()] processing confirmation: {}", confirmation_args);
            Ok(confirmation_args)
        } else {
            bail!("Received confirmation does not match the expected confirmation type")
        }
    }

    async fn fetch_next_confirmation(&self) -> Result<(Confirmation, Vec<u8>)> {
        let response = self.http_client.request(
            self.request_builder_confirm.fetch_next_confirmation()?
        ).await?;

        if response.status().is_success() {
            log::debug!("[fn fetch_next_confirmation()] StatusCode is successful: {}", response.status());
            let bytes = body::to_bytes(response.into_body()).await?;
            let confirmation = <Confirmation as BinaryPersist>::try_from_bytes(&bytes)?;
            Ok((confirmation, bytes.to_vec()))
        } else {
            log::error!("[fn fetch_next_confirmation()] HTTP Error. Status: {}", response.status());
            Ok((Confirmation::NO_CONFIRMATION, Vec::<u8>::default()))
        }
    }

    pub async fn subscribe_to_channel(&self, announcement_link_str: &str) -> Result<Subscription> {
        self.http_client.request(
            self.request_builder_command.subscribe_to_announcement(announcement_link_str)?
        ).await?;
        self.poll_confirmation::<Subscription>().await
    }

    pub async fn register_keyload_msg(&self, keyload_msg_link_str: &str) -> Result<KeyloadRegistration> {
        self.http_client.request(
            self.request_builder_command.register_keyload_msg(keyload_msg_link_str)?
        ).await?;
        self.poll_confirmation::<KeyloadRegistration>().await
    }

    pub async fn send_messages_in_endless_loop(&self, file_to_send: &str) -> Result<()> {
        self.http_client.request(
            self.request_builder_command.send_message_in_endless_loop(file_to_send)?
        ).await?;
        Ok(())
    }

    pub async fn println_subscriber_status(&self) -> Result<SubscriberStatus> {
        self.http_client.request(
            self.request_builder_command.println_subscriber_status()?
        ).await?;

        self.poll_confirmation::<SubscriberStatus>().await
    }

    pub async fn clear_client_state(&self)  -> Result<ClearClientState> {
        self.http_client.request(
            self.request_builder_command.clear_client_state()?
        ).await?;
        self.poll_confirmation::<ClearClientState>().await
    }

    pub async fn dev_eui_handshake(&self) -> Result<DevEuiHandshake> {
        let handshake_request_builder_command = RequestBuilderCommand::new(
            self.options.http_url.as_str(),
            self.options.dev_eui.as_str(),
            true
        );
        self.http_client.request(handshake_request_builder_command.dev_eui_handshake()?).await?;
        self.poll_confirmation::<DevEuiHandshake>().await
    }
}