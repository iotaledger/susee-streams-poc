use clap::Values;

use hyper::{
    Client,
    body,
    Body,
    client::HttpConnector,
    http::{
        StatusCode,
    },
};


use streams_tools::{
    STREAMS_TOOLS_CONST_HTTP_PROXY_URL,
    http_protocol_command::RequestBuilderCommand,
};

use anyhow::Result;
use std::fmt;

type HttpClient = Client<HttpConnector, Body>;

pub struct RemoteManagerOptions<'a> {
    pub http_url: &'a str,
}

impl Default for RemoteManagerOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_HTTP_PROXY_URL
        }
    }
}

impl fmt::Display for RemoteManagerOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RemoteManagerOptions: http_url: {}", self.http_url)
    }
}

pub struct RemoteManager<'a> {
    options: RemoteManagerOptions<'a>,
    http_client: HttpClient,
}

impl<'a> RemoteManager<'a> {

    pub fn new(options: Option<RemoteManagerOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        println!("[RemoteManager.new()] Initializing instance with options:\n{}\n", options);
        Self {
            options,
            http_client: HttpClient::new(),
        }
    }

    pub fn get_proxy_url(&self) -> &'a str { self.options.http_url }

    fn get_request_builder(&self) -> RequestBuilderCommand {
        RequestBuilderCommand::new(self.get_proxy_url())
    }

    pub async fn subscribe_to_channel(&self, announcement_link_str: &str) -> Result<()> {
        self.http_client.request(
            self.get_request_builder().subscribe_to_announcement(announcement_link_str)?
        ).await?;
        Ok(())
    }

    pub async fn register_keyload_msg(&self, keyload_msg_link_str: &str) -> Result<()> {
        self.http_client.request(
            self.get_request_builder().register_keyload_msg(keyload_msg_link_str)?
        ).await?;
        Ok(())
    }

    pub async fn send_messages(&self, mut files_to_send: Values<'_>) -> Result<()> {
        if let Some(first_file) = files_to_send.nth(0) {
            self.http_client.request(
                self.get_request_builder().send_message(first_file)?
            ).await?;
        } else {
            println!("[Sensor] WARNING: Could not find any filename in files_to_send list.");
        }
        Ok(())
    }

    pub async fn println_subscriber_status(&self) -> Result<()> {
        self.http_client.request(
            self.get_request_builder().println_subscriber_status()?
        ).await?;
        Ok(())
    }

    pub async fn clear_client_state(&self)  -> Result<()> {
        self.http_client.request(
            self.get_request_builder().clear_client_state()?
        ).await?;
        Ok(())
    }
}