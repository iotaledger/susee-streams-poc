#[cfg(feature = "std")]
pub mod std;

#[cfg(feature = "std")]
pub use self::std::main::process_main;

#[cfg(feature = "smol_rt")]
pub mod esp_r;

#[cfg(feature = "smol_rt")]
pub use self::esp_r::{
    main::process_main_esp_rs,
    http_client_smol_esp_rs::{
        HttpClient,
        HttpClientOptions,
    },
};
