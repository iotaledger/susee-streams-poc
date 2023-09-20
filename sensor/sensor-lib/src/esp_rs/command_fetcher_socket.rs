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

use std::fmt;

use log;

use streams_tools::{
    binary_persist::{
        Command,
    },
    http::{
        http_protocol_command::EndpointUris as EndpointUrisCommand,
    },
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
use async_trait::async_trait;

pub struct CommandFetcherSocketOptions<'a> {
    pub(crate) http_url: &'a str,
}

impl Default for CommandFetcherSocketOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL
        }
    }
}

impl fmt::Display for CommandFetcherSocketOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions: http_url: {}", self.http_url)
    }
}

pub struct CommandFetcherSocket<'a> {
    options: CommandFetcherSocketOptions<'a>,
}

impl<'a> CommandFetcherSocket<'a> {

    fn get_iota_bridge_path(&self, endpoint_uri: &str) -> String {
        format!("{}{}", self.options.http_url, endpoint_uri)
    }

    fn deserialize_command(& self, mut response: EspHttpResponse) -> Result<(Command, Vec<u8>)> {
        let mut ret_val = (Command::NO_COMMAND, Vec::<u8>::default());
        if let Some(content_len_u64) = response.content_len() {
            let content_len: usize = content_len_u64 as usize;
            log::debug!("[CommandFetcherSocket.deserialize_command] response.content_len()={}", content_len);
            let mut buffer = Vec::new();
            buffer.resize(content_len, 0);
            log::debug!("[CommandFetcherSocket.deserialize_command] do_read");
            (&mut response).read(&mut buffer)?;
            ret_val = deserialize_command(buffer)?;
        } else {
            log::error!("[CommandFetcherSocket.deserialize_command] response.content_len() is None");
        }
        Ok(ret_val)
    }
}

#[async_trait(?Send)]
impl<'a> CommandFetcher for CommandFetcherSocket<'a> {
    type Options = CommandFetcherSocketOptions<'a>;

    fn new(options: Option<CommandFetcherSocketOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[CommandFetcherSocket::new()] Creating new CommandFetcher using options: {}", options);
        Self {
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

        let headers = [("user-agent", "main-esp-rs/command-fetcher")];
        let url = self.get_iota_bridge_path(EndpointUrisCommand::FETCH_NEXT_COMMAND);

        let esp_http_req = http_client.request(
            embedded_svc::http::Method::Get,
            url.as_str(),
            &headers,
        )?;

        match esp_http_req.submit() {
            Ok(response) => {
                log::debug!("[CommandFetcherSocket.fetch_next_command] Received Response");
                if response.status() == HyperStatusCode::OK {
                    log::debug!("[CommandFetcherSocket.fetch_next_command] StatusCode::OK - deserializing command");
                    self.deserialize_command(response)
                } else {
                    log::error!("[CommandFetcherSocket.fetch_next_command] HTTP Error. Status: {}", response.status());
                    Ok((Command::NO_COMMAND, Vec::<u8>::default()))
                }
            },
            Err(e) => {
                bail!("[CommandFetcherSocket.fetch_next_command] esp_http_req.submit failed: {}", e)
            }
        }
    }

    async fn send_confirmation(&self, confirmation_request: HyperRequest<HyperBody>) -> Result<()> {
        let mut http_client = HyperEsp32Client::new(&Default::default(), UserAgentName::CommandFetcher);
        let response = http_client.send(confirmation_request).await?;
        log::debug!("[CommandFetcherSocket.send_confirmation] Received EspHttpResponse");
        if response.status == HyperStatusCode::OK {
            log::debug!("[CommandFetcherSocket.send_confirmation] StatusCode::OK");
            Ok(())
        } else {
            bail!("[CommandFetcherSocket.send_confirmation] Received HTTP Error as response for confirmation transmission. Status: {}", response.status)
        }
    }    
}