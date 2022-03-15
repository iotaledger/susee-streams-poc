
// Remove if STD is supported for your platform and you plan to use it
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

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


////////////////////////////////////////////////////////
// ESP-IDF                                            //
////////////////////////////////////////////////////////

#[no_mangle]
extern "C" fn app_main() {
    unsafe {
        let free_mem = esp_idf_sys::heap_caps_get_free_size(
            esp_idf_sys::MALLOC_CAP_8BIT
        );

        info!("heap_caps_get_free_size(MALLOC_CAP_8BIT): {}", free_mem);
    }

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
