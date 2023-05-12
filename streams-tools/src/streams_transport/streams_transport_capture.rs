use iota_streams::{
    app::transport::{
        Transport,
        TransportDetails,
        TransportOptions,
        tangle::{
            TangleAddress,
            TangleMessage,
            client::{
                Client,
                Details,
                SendOptions,
            }
        },
    },
    core::{
        async_trait,
        Result,
    },
};

use crate::compressed_state::{CompressedStateListen, CompressedStateSend};

use std::rc::Rc;

#[derive(Clone)]
pub struct StreamsTransportCapture(pub Client);

impl StreamsTransportCapture {
    pub fn new_from_url(url: &str) -> Self {
        Self(Client::new_from_url(url))
    }
}

impl CompressedStateSend for StreamsTransportCapture {
    fn subscribe_listener(&mut self, _listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
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
impl Transport<TangleAddress, TangleMessage> for StreamsTransportCapture
{
    /// Send a Streams message over the Tangle with the current timestamp and default SendOptions.
    async fn send_message(&mut self, msg: &TangleMessage) -> Result<()> {
        println!("\n[StreamsTransportCapture.send_message] Sending message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string());
        self.0.send_message(msg).await
    }

    /// Receive a message.
    async fn recv_messages(&mut self, link: &TangleAddress) -> Result<Vec<TangleMessage>> {
        let ret_val = self.0.recv_messages(link).await;
        match ret_val.as_ref() {
            Ok(msg_vec) => {
                for (idx, msg) in msg_vec.iter().enumerate() {
                    println!("[StreamsTransportCapture.recv_messages] - idx {}: Receiving message with {} bytes payload:\n{}\n", idx, msg.body.as_bytes().len(), msg.body.to_string())
                }
            },
            _ => ()
        }
        ret_val
    }

    async fn recv_message(&mut self, link: &TangleAddress) -> Result<TangleMessage> {
        let ret_val = self.0.recv_message(link).await;
        match ret_val.as_ref() {
            Ok(msg) => println!("[StreamsTransportCapture.recv_message] Receiving message with {} bytes payload:\n{}\n", msg.body.as_bytes().len(), msg.body.to_string()),
            _ => ()
        }
        ret_val
    }
}

#[async_trait(?Send)]
impl TransportDetails<TangleAddress> for StreamsTransportCapture {
    type Details = Details;
    async fn get_link_details(&mut self, link: &TangleAddress) -> Result<Self::Details> {
        self.0.get_link_details(link).await
    }
}

impl TransportOptions for StreamsTransportCapture {
    type SendOptions = SendOptions;
    fn get_send_options(&self) -> SendOptions {
        self.0.get_send_options()
    }
    fn set_send_options(&mut self, opt: SendOptions) {
        self.0.set_send_options(opt)
    }

    type RecvOptions = ();
    fn get_recv_options(&self) {}
    fn set_recv_options(&mut self, _opt: ()) {}
}
