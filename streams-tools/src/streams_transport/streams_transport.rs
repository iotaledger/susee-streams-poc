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
pub static STREAMS_TOOLS_CONST_INX_COLLECTOR_PORT: u16 = 9030;
pub static STREAMS_TOOLS_CONST_MINIO_DB_PORT: u16 = 9000;
pub static STREAMS_TOOLS_CONST_ANY_DEV_EUI: &str = "ANY";
pub static STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED: &str = "NOT_DEFINED";
// Time needed by the Transport layer (For example: IOTA network + inx-collector + minio) to process
// a block until it can be fetched from the database.
// The time is set to the 25 secs because it has turned out that this is the time needed for a block
// to be referenced by a milestone and to be processed by the streams-collector.
pub static STREAMS_TOOLS_CONST_TRANSPORT_PROCESSING_TIME_SECS: f32 = 25.0;

pub trait StreamsTransport: Clone + for <'a> Transport<'a, Msg = TransportMessage> + CompressedStateSend {
    type Options: Default;
    fn new(options: Option<Self::Options>) -> Self;
    fn set_initialization_cnt(&mut self, value: u8);
}
