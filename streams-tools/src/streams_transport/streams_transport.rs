use iota_streams::{
    app_channels::api::tangle::{
        Address,
        Message,
    },
    app::transport::{
        Transport,
        TransportDetails,
        TransportOptions,
    }
};

use crate::compressed_state::CompressedStateSend;

pub trait WrappedClient {
    fn new_from_url(url: &str) -> Self;
}

pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT: u16 = 50000;
pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL: &str = "http://localhost:50000";
pub static STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS: &str = "localhost:50001";


pub trait StreamsTransport: Clone + TransportOptions + Transport<Address, Message> + TransportDetails<Address> + CompressedStateSend {
    type Options: Default;
    fn new(options: Option<Self::Options>) -> Self;
    fn set_initialization_cnt(&mut self, value: u8);
}
