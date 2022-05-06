#![cfg_attr(not(feature = "std"), no_std)]

#![feature(asm)]
#![cfg_attr(target_arch = "xtensa", feature(asm_experimental_arch))]

#[cfg(not(feature = "std"))]
use core::panic::PanicInfo;

use log::*;
use sensor_lib::{
    process_main_esp_rs,
    HttpClient,
    HttpClientOptions,
};



/// Create aliases for FFI types for esp32c3, which doesn't have std.
#[cfg(not(feature = "std"))]
mod ffi {
    #![allow(dead_code)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]

    pub type c_char = u8;
    pub type c_int = i32;
}

// pub mod sys {
//     #![allow(non_upper_case_globals)]
//     #![allow(non_camel_case_types)]
//     #![allow(non_snake_case)]
//     include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
// }

fn create_tangle_http_client(options: Option<HttpClientOptions>) -> HttpClient {
    info!("create_tangle_http_client called");
    HttpClient::new(options)
}

#[no_mangle]
pub extern "C" fn process_main() -> i32 {

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

    0
}
