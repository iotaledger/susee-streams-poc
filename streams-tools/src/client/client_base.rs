pub trait WrappedClient {
    fn new_from_url(url: &str) -> Self;
}

pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT: u16 = 50000;
pub static STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL: &str = "http://localhost:50000";
pub static STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS: &str = "localhost:50001";
