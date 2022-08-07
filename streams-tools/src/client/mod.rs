pub mod client_base;

#[cfg(feature = "std")]
pub mod capture_client;
#[cfg(feature = "std")]
pub mod http_client;

pub use {
    client_base::{
        STREAMS_TOOLS_CONST_HTTP_PROXY_PORT,
        STREAMS_TOOLS_CONST_HTTP_PROXY_URL,
        WrappedClient,
    },
};

#[cfg(feature = "std")]
pub use {
    capture_client::CaptureClient,
    http_client::HttpClient,
};