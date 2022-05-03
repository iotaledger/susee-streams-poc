use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

// use smol::{io, net, prelude::*, Unblock};

use log::*;
use sensor_lib::{
    process_main_esp_rs,
};

fn main() -> Result<(), EspError> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    esp_idf_svc::log::EspLogger.set_target_level("Don't know what for this is used", LevelFilter::Trace);

    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        info!("heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }

    info!("Starting process_main()");

    match smol::block_on(async {
        info!("Start smol::block_on");
        process_main_esp_rs().await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("An error occurred while calling process_main(): {}", error);
        }
    };

    Ok(())
}