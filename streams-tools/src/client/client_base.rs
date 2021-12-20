pub trait WrappedClient {
    fn new_from_url(url: &str) -> Self;
}

pub static STREAMS_TOOLS_CONST_HTTP_PROXY_PORT: u16 = 50000;
pub static STREAMS_TOOLS_CONST_HTTP_PROXY_URL: &str = "http://localhost:50000";