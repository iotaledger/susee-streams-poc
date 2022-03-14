use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

// use smol::{io, net, prelude::*, Unblock};

use log::*;
use sensor_lib::{
    process_main_esp_rs,
    HttpClient,
    HttpClientOptions,
};

// use esp_idf_svc::{
//     // http::client::*,
//     timer::{
//         EspTimerService,
//         EspTimer,
//     }
// };


fn create_tangle_http_client(options: Option<HttpClientOptions>) -> HttpClient {
    info!("create_tangle_http_client called");
    HttpClient::new(options)
}

fn main() -> Result<(), EspError> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    esp_idf_svc::log::EspLogger.set_target_level("Don't know what for this is used", LevelFilter::Trace);

    info!("Starting process_main()");

    match smol::block_on(async {
        info!("Start smol::block_on");
        process_main_esp_rs(create_tangle_http_client).await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("An error occurred while calling process_main(): {}", error);
        }
    };

    Ok(())
}