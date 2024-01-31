use std::{
    fmt,
    cell::Cell
};

use hyper::{
    Client,
    body,
    Body,
    client::HttpConnector,
    http::{
        Response,
        request::{
            Request,
        },
    }
};

use anyhow::{
    Result,
    bail,
};

use streams_tools::{
    binary_persist::{
        BinaryPersist,
        Command,
        EnumeratedPersistableArgs,
        DevEuiHandshakeCmd,
    },
    http::{
        http_protocol_command::{
            RequestBuilderCommand
        },
    },
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    STREAMS_TOOLS_CONST_ANY_DEV_EUI,
    STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED
};

type HttpClient = Client<HttpConnector, Body>;

pub struct CommandFetcherOptions {
    pub http_url: String,
    // Controls if the CommandFetcher should do a DevEUI-Handshake
    // with the management-console before any other commands are fetched:
    // If true:
    //      The CommandFetcher will use the dev_eui 'ANY'
    //      (STREAMS_TOOLS_CONST_ANY_DEV_EUI) to fetch a DevEuiHandshake
    //      command. It will not accept any other type of command and will fetch
    //      commands until a DevEuiHandshake command has been received.
    //      After a DevEuiHandshake command has been received,
    //      the CommandFetcher will use the dev_eui defined by the
    //      'dev_eui' field of this struct.
    // If false (default):
    //      Will use the dev_eui defined by the 'dev_eui' field of this struct.
    pub dev_eui_handshake_first: bool,
    pub(crate) dev_eui: String
}

impl Default for CommandFetcherOptions {
    fn default() -> Self {
        Self {
            http_url: String::from(STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL),
            dev_eui_handshake_first: false,
            dev_eui: STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED.to_string(),
        }
    }
}

impl fmt::Display for CommandFetcherOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions:\n   http_url: {}\n   dev_eui_handshake_first: {}\n   dev_eui: {}",
               self.http_url,
               self.dev_eui_handshake_first,
               self.dev_eui,
        )
    }
}

pub struct CommandFetcher {
    options: CommandFetcherOptions,
    dev_eui_handshake_first: Cell<bool>,
    request_builder: RequestBuilderCommand,
}

impl CommandFetcher {

    pub fn new(options: Option<CommandFetcherOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[fn new()] Creating new CommandFetcher using options: {}", options);
        let http_url = options.http_url.clone();
        let dev_eui = if options.dev_eui_handshake_first {
            STREAMS_TOOLS_CONST_ANY_DEV_EUI.to_string()
        } else {
            options.dev_eui.clone()
        };
        Self {
            dev_eui_handshake_first: Cell::new(options.dev_eui_handshake_first),
            options,
            request_builder: RequestBuilderCommand::new(http_url.as_str(), dev_eui.as_str())
        }
    }

    async fn submit_request(&self) -> Response<Body> {
        let http_client = HttpClient::new();
        let request = self.request_builder.fetch_next_command()
            .expect("Error on creating http request");
        http_client.request(request).await
            .expect("Error on http_client.request")
    }

    pub async fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let response = self.submit_request().await;
        if response.status().is_success() {
            log::debug!("[fn fetch_next_command()] StatusCode is successful: {}", response.status());
            let (cmd, bytes) = self.deserialize_command(response).await?;
            if self.dev_eui_handshake_first.get() {
                if cmd == *DevEuiHandshakeCmd::INSTANCE {
                    log::info!("[fn fetch_next_command()] Received DevEuiHandshake command for dev_eui '{}'",
                        self.request_builder.get_dev_eui()
                    );
                    self.request_builder.set_dev_eui(self.options.dev_eui.as_str());
                    log::info!("[fn fetch_next_command()] dev_eui for next command fetches has been set to '{}'",
                        self.request_builder.get_dev_eui()
                    );
                    self.dev_eui_handshake_first.set(false);
                } else {
                    log::info!("[fn fetch_next_command()] Received command {} instead of DevEuiHandshake for dev_eui '{}' - \
                    will return Command::NO_COMMAND instead.",
                        cmd,
                        self.request_builder.get_dev_eui()
                    );
                    return Ok((Command::NO_COMMAND, Vec::<u8>::default()));
                }
            }
            Ok((cmd, bytes))
        } else {
            log::error!("[fn fetch_next_command()] HTTP Error. Status: {}", response.status());
            Ok((Command::NO_COMMAND, Vec::<u8>::default()))
        }
    }

    async fn deserialize_command(& self, response: Response<Body>) -> Result<(Command, Vec<u8>)> {
        let bytes = body::to_bytes(response.into_body()).await?;
        let command = <Command as BinaryPersist>::try_from_bytes(&bytes)?;
        Ok((command, bytes.to_vec()))
    }

    pub async fn send_confirmation(&self, confirmation_request: Request<Body>) -> Result<()> {
        let http_client = HttpClient::new();
        let response = http_client.request(confirmation_request).await?;
        log::debug!("[fn send_confirmation()] Received HttpResponse");
        if response.status().is_success() {
            log::debug!("[fn send_confirmation()] StatusCode is successful: {}", response.status());
            Ok(())
        } else {
            bail!("[fn send_confirmation()] Received HTTP Error as response for confirmation transmission. Status: {}", response.status())
        }
    }
}