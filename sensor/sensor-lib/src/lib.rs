pub mod command_fetcher;

#[cfg(feature = "std")]
pub mod std;

#[cfg(feature = "std")]
pub use self::std::main::process_main;

#[cfg(feature = "smol_rt")]
pub mod request_via_buffer_cb;
#[cfg(feature = "smol_rt")]
pub mod streams_poc_lib_api_types;

#[cfg(all(feature = "smol_rt", feature = "esp_idf"))]
pub mod esp_rs;

#[cfg(all(feature = "smol_rt", feature = "esp_idf"))]
pub use self::esp_rs::{
    main::{
        process_main_esp_rs,
        process_main_esp_rs_lwip,
    },
    streams_poc_lib,
    streams_transport_socket_esprs::{
        StreamsTransportSocketEspRs,
        StreamsTransportSocketEspRsOptions,
    },
};
