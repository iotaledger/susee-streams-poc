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
    }
};


use std::{
    clone::Clone,
    str::FromStr,
};

use hyper::{
    Body,
    http::{
        Response,
        Result,
    }
};

use crate::{
    binary_persist::{
        BinaryPersist,
        TANGLE_ADDRESS_BYTE_LEN,
    },
    http::http_protocol_streams::{
        ServerDispatchStreams,
        URI_PREFIX_STREAMS,
    },
    iota_bridge::helpers::log_err_and_respond_500,
};
use log;

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
[HttpClientProxy - DispatchStreams] send_message() - Incoming Message to attach to tangle with absolut length of {} bytes. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}

fn println_receive_message_from_address_for_received_message(message: &TangleMessage) {
    println!(
        "\
[HttpClientProxy - DispatchStreams] receive_message_from_address() - Received Message from tangle with absolut length of {} bytes. Data:
{}
", message.body.as_bytes().len() + LINK_AND_PREVLINK_LENGTH, message.to_string()
    );
}

#[async_trait(?Send)]
impl ServerDispatchStreams for DispatchStreams {

    fn get_uri_prefix(&self) -> &'static str { URI_PREFIX_STREAMS }

    async fn send_message<F: 'static + core::marker::Send + core::marker::Sync>(
        self: &mut Self, message: &TangleMessage) -> Result<Response<Body>>
    {
        println_send_message_for_incoming_message(message);
        let res = self.client.send_message(message).await;
        match res {
            Ok(_) => Ok(Response::new(Default::default())),
            Err(err) => log_err_and_respond_500(err, "send_message")
        }
    }

    async fn receive_message_from_address(self: &mut Self, address_str: &str) -> Result<Response<Body>> {
        log::debug!("[HttpClientProxy - DispatchStreams] receive_message_from_address() - Incoming request for address: {}", address_str);
        let address = TangleAddress::from_str(address_str).unwrap();
        let message = Transport::<TangleAddress, TangleMessage>::
            recv_message(&mut self.client, &address).await;
        match message {
            Ok(msg) => {
                println_receive_message_from_address_for_received_message(&msg);
                let mut buffer: Vec<u8> = vec![0;BinaryPersist::needed_size(&msg)];
                let size = BinaryPersist::to_bytes(&msg, buffer.as_mut_slice());
                log::debug!("[HttpClientProxy - DispatchStreams] receive_message_from_address() - Returning binary data via socket connection. length: {} bytes, data:\n\
{:02X?}\n", size.unwrap_or_default(), buffer);
                Ok(Response::new(buffer.into()))
            },
            Err(err) => log_err_and_respond_500(err, "[HttpClientProxy - DispatchStreams] receive_message_from_address()")
        }
    }

    async fn receive_messages_from_address(self: &mut Self, _address_str: &str) -> Result<Response<Body>> {
        unimplemented!()
    }

    async fn fetch_next_command(self: &mut Self) -> Result<Response<Body>> {
        unimplemented!()
    }
}