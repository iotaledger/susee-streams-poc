use hyper::{
    Body as HyperBody,
    http::{
        StatusCode as HyperStatusCode,
        Request as HyperRequest,
    }
};

use anyhow::{
    Result,
    bail,
};

use std::fmt;

use log;

use streams_tools::{
    binary_persist::{
        BinaryPersist,
        Command,
        HeaderFlags,
    },
    http::{
        http_protocol_command::EndpointUris as EndpointUrisCommand,
    },
};

use crate::{
    command_fetcher::{
        CommandFetcher,
        deserialize_command,
    },
    request_via_buffer_cb::{
        RequestViaBufferCallbackOptions,
        RequestViaBufferCallback
    }
};

use streams_tools::binary_persist::binary_persist_iota_bridge_req::{IotaBridgeResponseParts, IotaBridgeRequestParts, HttpMethod};
use async_trait::async_trait;

#[derive(Clone)]
pub struct CommandFetcherBufferCbOptions {
    pub(crate) buffer_cb: RequestViaBufferCallbackOptions,
}

impl Default for CommandFetcherBufferCbOptions {
    fn default() -> Self {
        Self {
            buffer_cb: RequestViaBufferCallbackOptions::default()
        }
    }
}

impl fmt::Display for CommandFetcherBufferCbOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions:
                buffer_cb: {}
                ",
            self.buffer_cb
        )
    }
}

pub struct CommandFetcherBufferCb {
    options: CommandFetcherBufferCbOptions,
}

impl CommandFetcherBufferCb {

    fn deserialize_command(& self, response: IotaBridgeResponseParts) -> Result<(Command, Vec<u8>)> {
        log::debug!("[fn deserialize_command()] response.body_bytes.len() = {}", response.body_bytes.len());
        deserialize_command(response.body_bytes)
    }
}

#[async_trait(?Send)]
impl CommandFetcher for CommandFetcherBufferCb {
    type Options = CommandFetcherBufferCbOptions;

    fn new(options: Option<CommandFetcherBufferCbOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Creating new CommandFetcher using options: {}", options);
        Self {
            options,
        }
    }

    fn get_iota_bridge_url(&self) -> Option<String> {
        None
    }

    async fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let mut request_buffer_cb = RequestViaBufferCallback::new(Some(self.options.buffer_cb.clone()));
        let url = format!("{}", EndpointUrisCommand::FETCH_NEXT_COMMAND); // May be "http://foor.bar/{}" is needed
        let header_flags = HeaderFlags::from(HttpMethod::GET);
        let request = IotaBridgeRequestParts::new(
            header_flags,
            url,
            Vec::<u8>::default(),
        );

        let request_bytes: Vec<u8> = request.as_vecu8()?;
        match request_buffer_cb.request_via_buffer_callback(request_bytes).await {
            Ok(response) => {
                log::debug!("[fn etch_next_command()] Received Response");
                if response.status_code == HyperStatusCode::OK {
                    log::debug!("[fn fetch_next_command()] StatusCode::OK - deserializing command");
                    self.deserialize_command(response)
                } else {
                    log::error!("[fn fetch_next_command()] HTTP Error. Status: {}", response.status_code);
                    Ok((Command::NO_COMMAND, Vec::<u8>::default()))
                }
            },
            Err(e) => {
                bail!("[fn fetch_next_command()] esp_http_req.submit failed: {}", e)
            }
        }
    }

    async fn send_confirmation(&self, confirmation_request: HyperRequest<HyperBody>) -> Result<()> {
        let mut request_buffer_cb = RequestViaBufferCallback::new(Some(self.options.buffer_cb.clone()));
        let request = IotaBridgeRequestParts::from_request(confirmation_request, false).await;
        let request_bytes = request.as_vecu8()?;
        let response = request_buffer_cb.request_via_buffer_callback(request_bytes).await?;
        log::debug!("[fn send_confirmation()] Received response");
        if response.status_code == HyperStatusCode::OK {
            log::debug!("[fn send_confirmation()] StatusCode::OK");
            Ok(())
        } else {
            bail!("[fn send_confirmation()] Received HTTP Error as response for confirmation transmission. Status: {}", response.status_code)
        }
    }
}