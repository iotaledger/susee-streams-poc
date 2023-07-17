use streams_tools::{
    binary_persist::{
        BinaryPersist,
        Command,
    },
    http::{
        http_protocol_command::EndpointUris,
        http_tools::RequestBuilderTools,
    },
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
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
use std::fmt;

type HttpClient = Client<HttpConnector, Body>;

pub struct CommandFetcherOptions {
    pub http_url: String,
}

impl Default for CommandFetcherOptions {
    fn default() -> Self {
        Self {
            http_url: String::from(STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL)
        }
    }
}

impl fmt::Display for CommandFetcherOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CommandFetcherOptions: http_url: {}", self.http_url)
    }
}

pub struct CommandFetcher {
    _options: CommandFetcherOptions,
    tools: RequestBuilderTools,
}

impl CommandFetcher {

    pub fn new(options: Option<CommandFetcherOptions>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[CommandFetcher::new()] Creating new CommandFetcher using options: {}", options);
        let http_url = options.http_url.clone();
        Self {
            _options: options,
            tools: RequestBuilderTools::new(http_url.as_str())
        }
    }

    fn get_request(&self) -> hyper::http::Result<Request<Body>> {
        RequestBuilderTools::get_request_builder()
            .method("GET")
            .uri(self.tools.get_uri(EndpointUris::FETCH_NEXT_COMMAND).as_str())
            .body(Body::empty())
    }

    async fn submit_request(&self) -> Response<Body> {
        let http_client = HttpClient::new();
        let request = self.get_request()
            .expect("Error on creating http request");
        http_client.request(request).await
            .expect("Error on http_client.request")
    }

    pub async fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let response = self.submit_request().await;
        if response.status().is_success() {
            log::debug!("[CommandFetcher.fetch_next_command] StatusCode is successful: {}", response.status());
            self.deserialize_command(response).await
        } else {
            log::error!("[CommandFetcher.fetch_next_command] HTTP Error. Status: {}", response.status());
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
        log::debug!("[CommandFetcher.send_confirmation] Received HttpResponse");
        if response.status().is_success() {
            log::debug!("[CommandFetcher.send_confirmation] StatusCode is successful: {}", response.status());
            Ok(())
        } else {
            bail!("[CommandFetcher.send_confirmation] Received HTTP Error as response for confirmation transmission. Status: {}", response.status())
        }
    }
}