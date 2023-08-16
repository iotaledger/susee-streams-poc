use lets::message::TransportMessage;
use streams::{
    transport::Transport,
};

use crate::compressed_state::CompressedStateSend;

pub trait WrappedClient {
    fn new_from_url(url: &str) -> Self;
}

pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT: u16 = 50000;
pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL: &str = "http://127.0.0.1:50000";
pub static STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS: &str = "127.0.0.1:50001";
pub static STREAMS_TOOLS_CONST_DEFAULT_BASE_BRANCH_TOPIC: &str = "MAIN";


pub trait StreamsTransport: Clone + for <'a> Transport<'a, Msg = TransportMessage> + CompressedStateSend {
    type Options: Default;
    fn new(options: Option<Self::Options>) -> Self;
    fn set_initialization_cnt(&mut self, value: u8);
}
