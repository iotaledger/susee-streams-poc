#[cfg(feature = "std")]
pub mod std;

#[cfg(feature = "std")]
pub use self::std::main::process_main;

#[cfg(feature = "smol_rt")]
pub mod esp_rs;

#[cfg(feature = "smol_rt")]
pub use self::esp_rs::{
    main::{
        process_main_esp_rs,
        process_main_esp_rs_wifi,
    },
    streams_poc_lib,
    http_client_smol_esp_rs::{
        HttpClientEspRs,
        HttpClientOptions,
    },
};
