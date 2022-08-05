
use embedded_svc::{
    io::Read,
    http::{
        Status,
        Headers,
        client::{
            Client,
            Request,
        }
    }
};

use esp_idf_svc::http::client::{
    EspHttpClient,
    EspHttpResponse
};

use streams_tools::{
    binary_persist::EnumeratedPersistable,
    binary_persist_command::{
        Command,
    },
    http_protocol_command::{
        EndpointUris,
    },
    STREAMS_TOOLS_CONST_HTTP_PROXY_URL,
    BinaryPersist
};

use hyper::http::StatusCode;

use anyhow::{
    Result,
    bail,
};
use std::fmt;
use log;

pub struct CommandFetcherOptions<'a> {
    pub(crate) http_url: &'a str,
}

impl Default for CommandFetcherOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: STREAMS_TOOLS_CONST_HTTP_PROXY_URL
        }
    }
}

impl fmt::Display for CommandFetcherOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions: http_url: {}", self.http_url)
    }
}

pub struct CommandFetcher<'a> {
    options: CommandFetcherOptions<'a>,
}

impl<'a> CommandFetcher<'a> {

    pub fn new(options: Option<CommandFetcherOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[CommandFetcher::new()] Creating new CommandFetcher using options: {}", options);
        Self {
            options,
        }
    }

    pub fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let mut http_client = EspHttpClient::new_default()?;
        let url = format!("{}{}", self.options.http_url, EndpointUris::FETCH_NEXT_COMMAND);

        let esp_http_req = http_client.request(
            embedded_svc::http::Method::Get,
            &url.to_string(),
        )?;

        match esp_http_req.submit() {
            Ok(response) => {
                log::debug!("[CommandFetcher.fetch_next_command] Received EspHttpResponse");
                if response.status() == StatusCode::OK {
                    log::debug!("[CommandFetcher.fetch_next_command] StatusCode::OK - deserializing command");
                    self.deserialize_command(response)
                } else {
                    log::error!("[CommandFetcher.fetch_next_command] HTTP Error. Status: {}", response.status());
                    Ok((Command::NO_COMMAND, Vec::<u8>::default()))
                }
            },
            Err(e) => {
                bail!("[CommandFetcher.fetch_next_command] esp_http_req.submit failed: {}", e)
            }
        }
    }

    fn deserialize_command(& self, mut response: EspHttpResponse) -> Result<(Command, Vec<u8>)> {
        let mut ret_val = (Command::NO_COMMAND, Vec::<u8>::default());
        if let Some(content_len) = response.content_len() {
            if content_len >= Command::LENGTH_BYTES {
                log::debug!("[CommandFetcher.deserialize_command] response.content_len()={}", content_len);
                let mut buffer = Vec::new();
                buffer.resize(content_len, 0);
                log::debug!("[CommandFetcher.deserialize_command] do_read");
                (&mut response).read(&mut buffer)?;
                log::debug!("[CommandFetcher.deserialize_command] create Command ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
                let command = Command::try_from_bytes(&buffer[0..Command::LENGTH_BYTES]).unwrap();
                log::debug!("[CommandFetcher.deserialize_command] return ret_val");
                ret_val = (command, buffer.to_vec());
            } else {
                log::error!("[CommandFetcher.deserialize_command] response.content_len() < Command::COMMAND_LENGTH_BYTES");
            }
        } else {
            log::error!("[CommandFetcher.deserialize_command] response.content_len() is None");
        }
        Ok(ret_val)
    }
}