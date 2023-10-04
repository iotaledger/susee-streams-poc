use std::{
    env,
    path::{
        Path,
        PathBuf
    }
};

fn main() {
    println!("streams_poc_lib - main - cargo_dir {}", env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("streams_poc_lib - main - target_dir {}", env::var("CARGO_BUILD_TARGET_DIR").unwrap());

    let cargo_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_dir = PathBuf::from(env::var("CARGO_BUILD_TARGET_DIR").unwrap());

    run_cbindgen(&cargo_dir, &target_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=path/to/Cargo.lock");
}

fn run_cbindgen(cargo_dir: &Path, target_dir: &Path) {
    let out = target_dir.join("streams_poc_lib.h");

    cbindgen::Builder::new()
        .with_crate(cargo_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("streams_poc_lib_h")
        .with_parse_deps(true)
        .with_parse_include(&["sensor-lib", "streams-tools"])
        .include_item("StreamsError")
        .include_item("LoRaWanError")
        .include_item("resolve_request_response_t")
        .include_item("send_request_via_lorawan_t")
        .include_item("iota_bridge_tcpip_proxy_options_t")
        .exclude_item("CREATE_NODE")
        .exclude_item("GET_NODE")
        .exclude_item("IS_NODE_KNOWN")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&out);

    println!("cargo:rerun-if-changed=src/lib.rs");
}
