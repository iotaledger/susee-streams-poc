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

type HttpClient = Client<HttpConnector, Body>;


pub struct RemoteManager {}

impl<'a> RemoteManager {
    pub fn get_proxy_url() -> &'static str { STREAMS_TOOLS_CONST_HTTP_PROXY_URL }

    fn get_request_builder() -> RequestBuilderCommand {
        RequestBuilderCommand::new(RemoteManager::get_proxy_url())
    }

    pub async fn subscribe_to_channel(announcement_link_str: &str) -> Result<()> {
        HttpClient::new().request(
            RemoteManager::get_request_builder().subscribe_to_announcement(announcement_link_str)?
        ).await?;
        Ok(())
    }

    pub async fn register_keyload_msg(keyload_msg_link_str: &str) -> Result<()> {
        HttpClient::new().request(
            RemoteManager::get_request_builder().register_keyload_msg(keyload_msg_link_str)?
        ).await?;
        Ok(())
    }

    pub async fn send_messages(mut files_to_send: Values<'_>) -> Result<()> {
        if let Some(first_file) = files_to_send.nth(0) {
            HttpClient::new().request(
                RemoteManager::get_request_builder().send_message(first_file)?
            ).await?;
        } else {
            println!("[Sensor] WARNING: Could not find any filename in files_to_send list.");
        }
        Ok(())
    }

    pub async fn println_subscriber_status() -> Result<()> {
        HttpClient::new().request(
            RemoteManager::get_request_builder().println_subscriber_status()?
        ).await?;
        Ok(())
    }
}