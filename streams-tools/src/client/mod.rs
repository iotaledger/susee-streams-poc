pub mod client_base;

#[cfg(feature = "std")]
pub mod capture_client;
#[cfg(feature = "std")]
pub mod http_client;

pub use {
    client_base::{
        STREAMS_TOOLS_CONST_IOTA_BRIDGE_PORT,
        STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
        STREAMS_TOOLS_CONST_DEFAULT_TCP_LISTENER_ADDRESS,
        WrappedClient,
    },
};

#[cfg(feature = "std")]
pub use {
    capture_client::CaptureClient,
    http_client::HttpClient,
};