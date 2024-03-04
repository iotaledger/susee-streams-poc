mod cli;

use std::{
    net::SocketAddr,
};

use anyhow::{
    Result,
    bail
};

use hyper::{
    Client,
    Body,
    body,
    client::HttpConnector,
};

use tokio::{
    net::{
        TcpListener,
        TcpStream,
    },
    io::{
        AsyncReadExt,
        AsyncWriteExt
    }
};

use streams_tools::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
    http::http_protocol_lorawan_rest::RequestBuilderLorawanRest,
    binary_persist::{
        BinaryPersist,
        USIZE_LEN,
        binary_persist_iota_bridge_req::IotaBridgeRequestParts,
    },
    LoraWanRestClientOptions,
};

use susee_tools::set_env_rust_log_variable_if_not_defined_by_env;

use cli::{
    LoraWanAppServerMockCli,
    ARG_KEYS,
    get_arg_matches,
};

const RECEIVE_IOTA_BRIDGE_REQUEST_BUFFER_SIZE: usize = 2048;

type HttpClient = Client<HttpConnector, Body>;

pub struct LoraWanRestClient {
    http_client: HttpClient,
    request_builder: RequestBuilderLorawanRest,
}

impl<'a> LoraWanRestClient {
    pub fn new(options: Option<LoraWanRestClientOptions<'a>>) -> Self {
        let options = options.unwrap_or_default();
        log::debug!("[LoraWanRestClient.new()] Initializing instance with options:\n       {}\n", options);
        Self {
            http_client: HttpClient::new(),
            request_builder: RequestBuilderLorawanRest::new(options.iota_bridge_url)
        }
    }

    pub async fn post_binary_request_to_iota_bridge(&self, request_bytes: Vec<u8>, dev_eui: &str) -> Result<Vec<u8>> {
        log::debug!("[LoraWanRestClient.post_binary_request_to_iota_bridge()] Sending {} bytes via lora-wan-rest http request to iota-bridge", request_bytes.len());
        let response = self.http_client.request(
            self.request_builder.post_binary_request(request_bytes, dev_eui)
                .expect("Error on building http requ\
                est for api function 'post_binary_request'")
        ).await.expect("Error on sending http request");

        if response.status().is_success() {
            log::debug!("[LoraWanRestClient.post_binary_request_to_iota_bridge] {:?} - Returning Bytes", response.status());
            let bytes = body::to_bytes(response.into_body()).await?;
            Ok(bytes.to_vec())
        } else {
            log::error!("[LoraWanRestClient.post_binary_request_to_iota_bridge] HTTP Error. Status: {}", response.status());
            let response_body_str = format!("Received Error {} from Iota-Bridge", response.status());
            Ok(response_body_str.into_bytes())
        }
    }
}

async fn handle_received_iota_bridge_request(stream: &mut TcpStream, dev_eui: u64, buf: &[u8], iota_bridge_url: &str) {
    println!("[LoraWanAppServerMock - fn handle_received_iota_bridge_request()] Received {} bytes to be send to iota-bridge {}", buf.len(), iota_bridge_url);
    let lorawan_rest_client = LoraWanRestClient::new(
        Some(
            LoraWanRestClientOptions{iota_bridge_url}
        )
    );

    let dev_eui_hex_str = format!("{:X}", dev_eui);
    match lorawan_rest_client.post_binary_request_to_iota_bridge(buf.to_vec(), dev_eui_hex_str.as_str()).await {
        Ok(response) => {
            println!("[LoraWanAppServerMock - fn handle_received_iota_bridge_request()] Received {} bytes from iota-bridge. Sending bytes via socket back to client",
                     response.len());
            stream
                .write_all(&response)
                .await
                .expect("failed to write data to socket");
            stream
                .flush().await.expect("failed to flush the TcpStream");
        }
        Err(_) => {
            log::error!("[LoraWanAppServerMock - fn handle_received_iota_bridge_request()] Received Err from lorawan_rest_client. Performing shutdown(Write).");
            // https://docs.rs/tokio/1.21.2/tokio/io/trait.AsyncWriteExt.html#method.shutdown
            // The TcpStream implementation will issue a shutdown(Write) sys call ...
            stream.shutdown().await
                .expect("stream.shutdown() returned an Err");
        }
    }
}

async fn receive_iota_bridge_request(stream: &mut TcpStream, dev_eui: u64, request_length: usize, address: &SocketAddr, iota_bridge_url: &str) -> Result<()>{
    // In case the request_length exceeds our read buffer size this test application just panics
    // with an appropriate error message.
    // In a production service implementation an additional loop should be used to read the stream
    // as long as the complete request has been received.
    if request_length > RECEIVE_IOTA_BRIDGE_REQUEST_BUFFER_SIZE {
        panic!("Please increase RECEIVE_IOTA_BRIDGE_REQUEST_BUFFER_SIZE - the buffer size of this test application.\n\
                    The Current data buffer size is {} bytes.", RECEIVE_IOTA_BRIDGE_REQUEST_BUFFER_SIZE)
    }
    let mut buf = [0 as u8; RECEIVE_IOTA_BRIDGE_REQUEST_BUFFER_SIZE];

    match stream.read_exact(&mut buf[0..request_length]).await {
        Ok(data_size) => {
            if request_length != data_size {
                bail!("Size of received data does not match the number of IotaBridgeRequest bytes probably because of a closed or erroneous connection.\n\
                    Current request length is {} bytes but {} bytes have been received.\n\
                    Will return an error to stop the message loop.", request_length, data_size)
            }
            handle_received_iota_bridge_request(stream, dev_eui, &buf[0..request_length], iota_bridge_url).await;
            Ok(())
        }
        Err(e) => {
            bail!("An error occurred while reading a IotaBridgeRequest of {} bytes, terminating connection with {}. Error: {}", request_length, address, e)
        }
    }
}

async fn read_request_length_and_process_iota_bridge_request(stream: &mut TcpStream, dev_eui: u64, address: &SocketAddr, iota_bridge_url: &str) -> Result<()>{
    let mut request_size_buffer = [0; USIZE_LEN];
    match stream.peek(&mut request_size_buffer).await {
        Ok(bytes_received) => {
            if bytes_received == USIZE_LEN {
                let request_length = IotaBridgeRequestParts::get_request_byte_size(&request_size_buffer).expect("Error on deserializing request_byte_size");
                println!("             IotaBridgeRequest has {} bytes of data", request_length);
                receive_iota_bridge_request(stream, dev_eui, request_length, address, iota_bridge_url).await?;
                Ok(())
            } else {
                bail!("[LoraWanAppServerMock - main()] Received only {} bytes while reading request size.", bytes_received);
            }
        }
        Err(e) => {
            bail!("An error occurred while reading request size. Error: {}", e);
        }
    }
}

const DEV_EUI_LEN_BYTES: usize = 8;

async fn handle_new_tcp_connection(stream: &mut TcpStream, address: &SocketAddr, iota_bridge_url: &str) {
    // In this test application we are using a buffer that is larger than all requests
    // occurring in real world usage. In a production service implementation a loop should be used
    // to read the stream as long as a complete IotaBridgeRequestParts has been received.
    println!("New connection to client {}. Starting message loop.", address);

    loop {
        // The LoraWAN DevEUI is prepended as u64 value
        let mut dev_eui_read_buffer = [0; DEV_EUI_LEN_BYTES];
        match stream.read(&mut dev_eui_read_buffer).await {
            Ok(bytes_received) => {
                if bytes_received == DEV_EUI_LEN_BYTES {
                    let dev_eui = u64::try_from_bytes(&dev_eui_read_buffer).expect("Error on deserializing dev_eui");
                    println!("Received new IotaBridgeRequest from LoraWAN Node DevEUI {}", dev_eui);
                    match read_request_length_and_process_iota_bridge_request(stream, dev_eui, address, iota_bridge_url).await {
                        Ok(_) => {}
                        Err(e) => {
                            log::warn!("[LoraWanAppServerMock - main()] Received an error from handle_new_iota_bridge_request(). Ending message loop for client {}. Error: {}",
                                       address, e);
                            break;
                        }
                    }
                } else {
                    log::warn!("[LoraWanAppServerMock - main()] Received only {} bytes while reading request size. Ending message loop for client {}.",
                               bytes_received, address);
                    break;
                }
            }
            Err(e) => {
                println!("An error occurred while reading dev_eui, terminating connection with {}. Error: {}", address, e);
                stream.shutdown().await.expect("stream.shutdown() returned an Err");
                break;
            }
        }
    }
}

async fn run_tcp_listener_loop(addr_str: &str, iota_bridge_url: &str) {
    let listener = TcpListener::bind(&addr_str).await
        .expect(format!("Could not bind to address: '{}'", addr_str).as_str());

    println!("Listening on: {}", addr_str);

    loop {
        let (mut socket, address) = listener.accept().await
            .expect("listener.accept() returned an Err");

        let iota_bridge_url_cloned= String::from(iota_bridge_url);
        tokio::spawn(async move {
            handle_new_tcp_connection(&mut socket, &address, iota_bridge_url_cloned.as_str()).await;
        });
    }
}

#[tokio::main]
async fn main() {
    set_env_rust_log_variable_if_not_defined_by_env("info");
    env_logger::init();
    let matches_and_options = get_arg_matches();
    let cli = LoraWanAppServerMockCli::new(&matches_and_options, &ARG_KEYS) ;

    let addr_str = cli.matches.value_of(cli.arg_keys.listener_ip_address_port).unwrap().trim();

    let iota_bridge_url = if cli.matches.is_present(cli.arg_keys.iota_bridge_url) {
        cli.matches.value_of(cli.arg_keys.iota_bridge_url).unwrap().trim()
    } else {
        STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL
    };

    run_tcp_listener_loop(addr_str, iota_bridge_url).await;
}
