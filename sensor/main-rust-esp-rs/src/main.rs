use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_sys::EspError;

use log::*;
use sensor_lib::{
    process_main_esp_rs_wifi,
};

const WIFI_SSID: &str = env!("SENSOR_MAIN_POC_WIFI_SSID");
const WIFI_PASS: &str = env!("SENSOR_MAIN_POC_WIFI_PASS");
const IOTA_BRIDGE_URL: &str = env!("SENSOR_MAIN_POC_IOTA_BRIDGE_URL");

fn main() -> Result<(), EspError> {
    // Called to ensure that needed esp_idf_sys patches are linked to our binary.
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    // https://docs.espressif.com/projects/esp-idf/en/latest/esp32/api-reference/system/log.html#_CPPv417esp_log_level_setPKc15esp_log_level_t
    // esp_idf_svc::log::EspLogger.set_target_level("*", LevelFilter::Trace);

    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        info!("heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }

    info!("Starting process_main()");

    match smol::block_on(async {
        info!("Start smol::block_on");
        process_main_esp_rs_wifi(WIFI_SSID, WIFI_PASS, IOTA_BRIDGE_URL, None).await
    }){
        Ok(_) => {},
        Err(error) => {
            error!("An error occurred while calling process_main(): {}", error);
        }
    };

    Ok(())
}