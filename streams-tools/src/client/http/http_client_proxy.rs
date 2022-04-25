use iota_streams::{
    app::{
        transport::{
            Transport,
            tangle::{
                TangleAddress,
                TangleMessage,
                client::{
                    Client,
                }
            },
        },
    },
    core::{
        async_trait,
        Errors,
    }
};


use std::{
    clone::Clone,
    str::FromStr,
};

use hyper::{
    Body,
    http::{
        Request,
        Response,
        Result,
    }
};

use crate::{
    binary_persist::BinaryPersist,
    binary_persist_command::Command,
    http_protocol_streams::{
        ServerDispatchStreams,
        MapStreamsErrors,
    },
    http_protocol_command::{
        ServerDispatchCommand,
    },
    http_server_dispatch::dispatch_request
};
use std::collections::VecDeque;

#[derive(Clone)]
pub struct DispatchStreams {
    client: Client,
}

impl DispatchStreams
{
    pub fn new(client: &Client) -> Self {
        Self {
            client: client.clone(),
        }
    }
}

static LINK_AND_PREVLINK_LENGTH: usize = 2 * TANGLE_ADDRESS_BYTE_LEN;

fn println_send_message_for_incoming_message(message: &TangleMessage) {
    println!(
        "\
[HttpClientProxy - ServerDispatch - send_message] Incoming Message to attach to tangle with {} bytes payload. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}

fn println_receive_message_from_address_for_received_message(message: &TangleMessage) {
    println!(
        "\
[HttpClientProxy - ServerDispatch - receive_message_from_address] Received Message from tangle with {} bytes payload. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}


#[async_trait(?Send)]
impl ServerDispatchStreams for DispatchStreams {
    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage) -> Result<Response<Body>>
    {
        println_send_message_for_incoming_message(message);
        let res = self.client.send_message(message).await;
        match res {
            Ok(_) => Ok(Response::new(Default::default())),
            Err(err) => HttpClientProxy::log_err_and_respond_500(err, "send_message")
        }
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        println!("[HttpClientProxy - DispatchStreams] receive_message_from_address() - Incoming request for address: {}", address_str);
        let address = TangleAddress::from_str(address_str).unwrap();
        let message = Transport::<TangleAddress, TangleMessage>::
            recv_message(&mut self.client, &address).await;
        match message {
            Ok(msg) => {
                println_receive_message_from_address_for_received_message(&msg);
                let mut buffer: Vec<u8> = vec![0;BinaryPersist::needed_size(&msg)];
                let size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                println!("[HttpClientProxy - ServerDispatch - receive_message_from_address] Returning binary data via socket connection. length: {} bytes, data:\n\
{:02X?}\n", size.unwrap_or_default(), buffer);
                Ok(Response::new(buffer.into()))
            },
            Err(err) => HttpClientProxy::log_err_and_respond_500(err, "[HttpClientProxy - DispatchStreams] receive_message_from_address()")
        }
    }

    async fn receive_messages_from_address(self: &mut Self, _address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn fetch_next_command(self: &mut Self) -> Result<Response<Body>> {
        unimplemented!()
    }
}

#[derive(Clone)]
pub struct DispatchCommand {
    client: Client,
    fifo: VecDeque<Vec<u8>>,
}

impl DispatchCommand
{
    pub fn new(client: &Client) -> Self {
        Self {
            client: client.clone(),
            fifo: VecDeque::<Vec<u8>>::new(),
        }
    }
}

impl DispatchCommand {
    fn register_remote_command(self: &mut Self, req_body_binary: &[u8], api_fn_name: &str) -> Result<Response<Body>> {
        self.fifo.push_back(req_body_binary.to_vec());
        println!("[HttpClientProxy - DispatchCommand] {}() - Receiving command blob. Blob length {}", api_fn_name, req_body_binary.len());
        Ok(Response::new(Default::default()))
    }
}


#[async_trait(?Send)]
impl ServerDispatchCommand for DispatchCommand {
    async fn fetch_next_command(self: &mut Self) -> Result<Response<Body>> {
        if let Some(req_body_binary) = self.fifo.pop_front() {
            println!("[HttpClientProxy - DispatchCommand] fetch_next_command() - Returning command blob. Blob length {}", req_body_binary.len());
            Ok(Response::new(req_body_binary.into()))
        } else {
            println!("[HttpClientProxy - DispatchCommand] fetch_next_command() - No command available. Returning Command::NO_COMMAND.");
            let mut buffer: [u8; Command::COMMAND_LENGTH_BYTES] = [0; Command::COMMAND_LENGTH_BYTES];
            Command::NO_COMMAND.to_bytes(&mut buffer)?;
            Ok(Response::new(buffer.into()))
        }
    }

    async fn send_message(self: &mut Self, req_body_binary: &[u8]) -> Result<Response<Body>> {
        self.register_remote_command(req_body_binary, "send_message")
    }

    async fn subscribe_to_announcement(self: &mut Self, req_body_binary: &[u8]) -> Result<Response<Body>> {
        self.register_remote_command(req_body_binary, "subscribe_to_announcement")
    }

    async fn register_keyload_msg(self: &mut Self, req_body_binary: &[u8]) -> Result<Response<Body>> {
        self.register_remote_command(req_body_binary, "register_keyload_msg")
    }

    async fn println_subscriber_status(self: &mut Self, req_body_binary: &[u8]) -> Result<Response<Body>> {
        self.register_remote_command(req_body_binary, "println_subscriber_status")
    }
}

#[derive(Clone)]
pub struct HttpClientProxy {
    dispatch_streams: DispatchStreams,
    dispatch_command: DispatchCommand,
}

impl HttpClientProxy
{
    pub fn new_from_url(url: &str) -> Self {
        let client = Client::new_from_url(url);
        Self {
            dispatch_streams: DispatchStreams::new(&client),
            dispatch_command: DispatchCommand::new(&client),
        }
    }

    pub async fn handle_request(&mut self, req: Request<Body>) -> Result<Response<Body>> {
        dispatch_request(req, &mut self.dispatch_streams, &mut self.dispatch_command).await
    }
}


impl HttpClientProxy {
    fn log_err_and_respond_500(err: anyhow::Error, fn_name: &str) -> Result<Response<Body>> {
        println!("[HttpClientProxy - {}] Error: {}", fn_name, err);

        // // Following implementation does not work because currently it is not possible to access
        // // The streams error value. Instead we expect a MessageLinkNotFoundInTangle error to
        // // make the susee POC run at all.
        // // TODO: Check how to access the streams error value and fix the implementation here
        // let streams_error = &MapStreamsErrors::get_indicator_for_uninitialized();
        // for cause in err.chain() {
        //     if let Some(streams_err) = cause.downcast_ref::<Errors>() {
        //         streams_error = streams_err.clone();
        //         break;
        //     }
        // }
        // let mut status_code = MapStreamsErrors::to_http_status_codes(&streams_error);

        let status_code = MapStreamsErrors::to_http_status_codes(&Errors::MessageLinkNotFoundInTangle(String::from("")));
        let builder = Response::builder()
            .status(status_code);
        builder.body(Default::default())
    }
}
