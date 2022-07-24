use std::env;
use std::path::{Path, PathBuf};

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
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&out);

    println!("cargo:rerun-if-changed=src/lib.rs");
}
