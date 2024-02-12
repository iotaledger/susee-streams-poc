use std::fmt;

use async_trait::async_trait;

use hyper::{
    Body as HyperBody,
    http::{
        StatusCode as HyperStatusCode,
        Request as HyperRequest,
    }
};

use embedded_svc::{
    http::{
        Headers,
        client::{
            Response,
            Client as HttpClient,
        }
    }
};

use esp_idf_svc::http::client::{
    EspHttpConnection,
};

use anyhow::{
    Result,
    bail,
};

use log;

use streams_tools::{
    binary_persist::{
        Command,
    },
    http::http_protocol_command::RequestBuilderCommand,
    STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED,
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
};

use crate::{
    command_fetcher::{
        CommandFetcher,
        deserialize_command,
    },
    esp_rs::{
        hyper_esp_rs_tools::{
            HyperEsp32Client,
            EspHttpResponse,
            UserAgentName,
        }
    }
};

const HEADERS: [(&'static str, &'static str); 1] = [("user-agent", "main-esp-rs/command-fetcher")];

pub struct CommandFetcherSocketOptions {
    pub(crate) http_url: String,
    // Controls if the CommandFetcher should do a DevEUI-Handshake first.
    // See RequestBuilderCommand::dev_eui_handshake_first in streams-tools/src/http/http_protocol_command.rs
    // for more details.
    pub dev_eui_handshake_first: bool,
    pub dev_eui: String
}

impl Default for CommandFetcherSocketOptions{
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL.to_string(),
            dev_eui_handshake_first: false,
            dev_eui: STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED.to_string(),
        }
    }
}

impl fmt::Display for CommandFetcherSocketOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions:\n   http_url: {}\n   dev_eui_handshake_first: {}\n   dev_eui: {}",
                self.http_url,
                self.dev_eui_handshake_first,
                self.dev_eui
        )
    }
}

pub struct CommandFetcherSocket {
    options: CommandFetcherSocketOptions,
    request_builder: RequestBuilderCommand,
}

impl CommandFetcherSocket {

    fn deserialize_command(& self, mut response: EspHttpResponse) -> Result<(Command, Vec<u8>)> {
        let mut ret_val = (Command::NO_COMMAND, Vec::<u8>::default());
        if let Some(content_len_u64) = response.content_len() {
            let content_len: usize = content_len_u64 as usize;
            log::debug!("[fn deserialize_command()] response.content_len()={}", content_len);
            let mut buffer = Vec::new();
            buffer.resize(content_len, 0);
            log::debug!("[fn deserialize_command()] do_read");
            (&mut response).read(&mut buffer)?;
            ret_val = deserialize_command(buffer)?;
        } else {
            log::error!("[fn deserialize_command()] response.content_len() is None");
        }
        Ok(ret_val)
    }

    fn get_request_uri(&self) -> Result<String> {
        let hyper_request = self.request_builder.fetch_next_command()?;
        Ok(hyper_request.uri().to_string())
    }

    async fn handle_response(&self, response: Response<&mut EspHttpConnection>) -> Result<(Command, Vec<u8>)> {
        if response.status() == HyperStatusCode::OK {
            log::debug!("[fn handle_response()] StatusCode::OK - deserializing command");
            let (cmd, bytes) = self.deserialize_command(response)?;
            match self.request_builder.manage_dev_eui_by_received_command(&cmd) {
                Ok(_) => Ok((cmd, bytes)),
                Err(e) => {
                    log::error!("[fn handle_response()] Received unexpected command: {} - will return Command::NO_COMMAND instead.",
                        e
                    );
                    Ok((Command::NO_COMMAND, Vec::<u8>::default()))
                }
            }
        } else {
            log::error!("[fn handle_response()] HTTP Error. Status: {}", response.status());
            Ok((Command::NO_COMMAND, Vec::<u8>::default()))
        }
    }
}

#[async_trait(?Send)]
impl CommandFetcher for CommandFetcherSocket {
    type Options = CommandFetcherSocketOptions;

    fn new(options: Option<CommandFetcherSocketOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Creating new CommandFetcher using options: {}", options);
        Self {
            request_builder: RequestBuilderCommand::new(
                options.http_url.as_str(),
                options.dev_eui.as_str(),
                options.dev_eui_handshake_first,
            ),
            options,
        }
    }

    fn get_iota_bridge_url(&self) -> Option<String> {
        Some(self.options.http_url.to_string())
    }

    async fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let mut http_client = HttpClient::wrap(
            EspHttpConnection::new(&Default::default())?
        );

        let uri = self.get_request_uri()?;
        let esp_http_req = http_client.request(
            embedded_svc::http::Method::Get,
            uri.as_str(),
            &HEADERS,
        )?;

        match esp_http_req.submit() {
            Ok(response) => {
                log::debug!("[fn fetch_next_command()] Received Response");
                self.handle_response(response).await
            },
            Err(e) => {
                bail!("[fn fetch_next_command()] esp_http_req.submit failed: {}", e)
            }
        }
    }

    async fn send_confirmation(&self, confirmation_request: HyperRequest<HyperBody>) -> Result<()> {
        let mut http_client = HyperEsp32Client::new(&Default::default(), UserAgentName::CommandFetcher);
        let response = http_client.send(confirmation_request).await?;
        log::debug!("[fn send_confirmation] Received EspHttpResponse");
        if response.status == HyperStatusCode::OK {
            log::debug!("[fn send_confirmation()] StatusCode::OK");
            Ok(())
        } else {
            bail!("[fn send_confirmation()] Received HTTP Error as response for confirmation transmission. Status: {}", response.status)
        }
    }    
}