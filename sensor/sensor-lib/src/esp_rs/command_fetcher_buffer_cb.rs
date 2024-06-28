use std::fmt;

use async_trait::async_trait;

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

use log;

use streams_tools::{
    binary_persist::{
        BinaryPersist,
        Command,
        HeaderFlags,
        binary_persist_iota_bridge_req::{
            IotaBridgeResponseParts,
            IotaBridgeRequestParts,
            HttpMethod
        }
    },
    http::http_protocol_command::RequestBuilderCommand,
    STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED,
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

#[derive(Clone)]
pub struct CommandFetcherBufferCbOptions {
    pub(crate) buffer_cb: RequestViaBufferCallbackOptions,
    // Controls if the CommandFetcher should do a DevEUI-Handshake first.
    // See RequestBuilderCommand::dev_eui_handshake_first in streams-tools/src/http/http_protocol_command.rs
    // for more details.
    pub dev_eui_handshake_first: bool,
    pub dev_eui: String
}

impl Default for CommandFetcherBufferCbOptions {
    fn default() -> Self {
        Self {
            buffer_cb: RequestViaBufferCallbackOptions::default(),
            dev_eui_handshake_first: false,
            dev_eui: STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED.to_string(),
        }
    }
}

impl fmt::Display for CommandFetcherBufferCbOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions:\n   dev_eui_handshake_first: {}\n   dev_eui: {}\n   buffer_cb: {}",
               self.dev_eui_handshake_first,
               self.dev_eui,
               self.buffer_cb
        )
    }
}

pub struct CommandFetcherBufferCb {
    options: CommandFetcherBufferCbOptions,
    request_builder: RequestBuilderCommand,
}

impl CommandFetcherBufferCb {

    fn deserialize_command(& self, response: IotaBridgeResponseParts) -> Result<(Command, Vec<u8>)> {
        log::debug!("[fn deserialize_command()] response.body_bytes.len() = {}", response.body_bytes.len());
        deserialize_command(response.body_bytes)
    }

    fn get_request_uri(&self) -> Result<String> {
        let hyper_request = self.request_builder.fetch_next_command()?;
        Ok(hyper_request.uri().to_string())
    }

    fn handle_response(&self, response: IotaBridgeResponseParts) -> Result<(Command, Vec<u8>)> {
        if response.status_code == HyperStatusCode::OK {
            log::debug!("[fn fetch_next_command()] StatusCode::OK - deserializing command");
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
            log::error!("[fn handle_response()] HTTP Error. Status: {}", response.status_code);
            Ok((Command::NO_COMMAND, Vec::<u8>::default()))
        }
    }
}

#[async_trait(?Send)]
impl CommandFetcher for CommandFetcherBufferCb {
    type Options = CommandFetcherBufferCbOptions;

    fn new(options: Option<CommandFetcherBufferCbOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Creating new CommandFetcher using options: {}", options);
        Self {
            // We do not specify a uri_prefix here because the Application-Server_connector will decide which IOTA-Bridge is used
            // for the post_binary_request call (lorawan_rest_request) and the IOTA-Bridge wil use the request path to dispatch the
            // request internally without any domain name.
            request_builder: RequestBuilderCommand::new(
                "",
                options.dev_eui.as_str(),
                options.dev_eui_handshake_first,
            ),
            options,
        }
    }

    fn get_iota_bridge_url(&self) -> Option<String> {
        None
    }

    async fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let mut request_buffer_cb = RequestViaBufferCallback::new(Some(self.options.buffer_cb.clone()));
        let uri = self.get_request_uri()?;
        let header_flags = HeaderFlags::from(HttpMethod::GET);
        let request = IotaBridgeRequestParts::new(
            header_flags,
            uri,
            Vec::<u8>::default(),
        );

        let request_bytes: Vec<u8> = request.as_vecu8()?;
        match request_buffer_cb.request_via_buffer_callback(request_bytes).await {
            Ok(response) => {
                log::debug!("[fn fetch_next_command()] Received Response");
                self.handle_response(response)
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