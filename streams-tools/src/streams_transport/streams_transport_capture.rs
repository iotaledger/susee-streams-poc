use std::rc::Rc;

use async_trait::async_trait;

use anyhow::Result as AnyResult;

use streams::{
    Address,
    transport::Transport,
};

use lets::{
    error::Result,
    message::TransportMessage,
    transport::tangle::{
        Client,
    },
};
use crate::{
    binary_persist::{
        trans_msg_encode,
        trans_msg_len
    },
    compressed_state::{
        CompressedStateListen,
        CompressedStateSend
    },
    user_manager::message_indexer::{
        MessageIndexer,
        MessageIndexerOptions,
    },
    helpers::get_iota_node_url,
};

pub struct StreamsTransportCapture(pub Client<MessageIndexer>);

impl StreamsTransportCapture {
    pub async fn new_from_url(iota_node: &str) -> Self {
        let indexer = MessageIndexer::new(MessageIndexerOptions::new(iota_node.to_string()));
        Self(Client::for_node(
            &get_iota_node_url(iota_node),
            indexer
        ).await.expect("Error on creating Client"))
    }
}

impl CompressedStateSend for StreamsTransportCapture {
    fn subscribe_listener(&mut self, _listener: Rc<dyn CompressedStateListen>) -> AnyResult<usize> {
        unimplemented!()
    }

    fn set_initial_use_compressed_msg_state(&self, _use_compressed_msg: bool) {
        unimplemented!()
    }

    fn remove_listener(&mut self, _handle: usize) {
        unimplemented!()
    }
}

#[async_trait(?Send)]
impl<'a> Transport<'a> for StreamsTransportCapture
{
    type Msg = TransportMessage;
    type SendResponse = TransportMessage;

    /// Send a Streams message over the Tangle with the current timestamp and default SendOptions.
    async fn send_message(&mut self, address: Address, msg: Self::Msg) -> Result<Self::SendResponse> {
        log::info!("\n[fn send_message()] Sending message with {} bytes payload:\n{}\n", trans_msg_len(&msg), trans_msg_encode(&msg));
        self.0.send_message(address, msg).await
    }

    /// Receive a message.
    async fn recv_messages(&mut self, address: Address) -> Result<Vec<Self::Msg>> {
        let ret_val = self.0.recv_messages(address).await;
        match ret_val.as_ref() {
            Ok(msg_vec) => {
                for (idx, msg) in msg_vec.iter().enumerate() {
                    log::info!("[fn recv_messages()] - idx {}: Receiving message with {} bytes payload:\n{}\n", idx, trans_msg_len(&msg), trans_msg_encode(&msg))
                }
            },
            _ => ()
        }
        ret_val
    }
}
