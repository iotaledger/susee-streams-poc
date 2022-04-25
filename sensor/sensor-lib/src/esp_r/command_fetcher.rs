
use embedded_svc::http::{
    Status,
    client::{
        Client,
        Request,
    }
};

use esp_idf_svc::http::client::{
    EspHttpClient,
    EspHttpResponse
};

use streams_tools::{
    binary_persist_command::{
        Command,
    },
    http_protocol_command::{
        EndpointUris
    }
};

use hyper::http::StatusCode;

use anyhow::{
    Result,
    bail,
};
use embedded_svc::io::Read;
use embedded_svc::http::Headers;

pub struct CommandFetcherOptions<'a> {
    http_url: &'a str,
}

impl Default for CommandFetcherOptions<'_> {
    fn default() -> Self {
        Self {
            http_url: "http://192.168.38.69:50000"
        }
    }
}

pub struct CommandFetcher<'a> {
    options: CommandFetcherOptions<'a>,
}

impl<'a> CommandFetcher<'a> {

    pub fn new(options: Option<CommandFetcherOptions<'a>>) -> Self {
        Self {
            options: options.unwrap_or_default(),
        }
    }

    pub fn fetch_next_command(& self) -> Result<(Command, Vec<u8>)> {
        let mut http_client = EspHttpClient::new_default()?;
        let url = format!("{}{}", self.options.http_url, EndpointUris::FETCH_NEXT_COMMAND);

        let esp_http_req = http_client.request(
            embedded_svc::http::Method::Get,
            url.to_string(),
        )?;

        match esp_http_req.submit() {
            Ok(response) => {
                println!("[CommandFetcher.fetch_next_command] Received EspHttpResponse");
                if response.status() == StatusCode::OK {
                    println!("[CommandFetcher.fetch_next_command] StatusCode::OK - deserializing command");
                    self.deserialize_command(response)
                } else {
                    println!("[CommandFetcher.fetch_next_command] HTTP Error. Status: {}", response.status());
                    Ok((Command::NO_COMMAND, Vec::<u8>::default()))
                }
            },
            Err(e) => {
                bail!("[CommandFetcher.fetch_next_command] esp_http_req.submit failed: {}", e)
            }
        }
    }

    fn deserialize_command(& self, response: EspHttpResponse) -> Result<(Command, Vec<u8>)> {
        let mut ret_val = (Command::NO_COMMAND, Vec::<u8>::default());
        if let Some(content_len) = response.content_len() {
            if content_len >= Command::COMMAND_LENGTH_BYTES {
                println!("[CommandFetcher.deserialize_command] response.content_len()={}", content_len);
                let mut buffer = Vec::new();
                buffer.resize(content_len, 0);
                println!("[CommandFetcher.deserialize_command] do_read");
                (&response).do_read(&mut buffer)?;
                println!("[CommandFetcher.deserialize_command] create Command ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}", buffer.len(), buffer.as_slice());
                let command = Command::from_bytes(&buffer[0..Command::COMMAND_LENGTH_BYTES]).unwrap();
                println!("[CommandFetcher.deserialize_command] return ret_val");
                ret_val = (command, buffer.to_vec());
            } else {
                println!("[CommandFetcher.deserialize_command] response.content_len() < Command::COMMAND_LENGTH_BYTES");
            }
        } else {
            println!("[CommandFetcher.deserialize_command] response.content_len() is None");
        }
        Ok(ret_val)
    }
}