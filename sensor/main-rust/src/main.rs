// #![no_std]
use anyhow::Result;
use sensor_lib::{process_main};
use susee_tools::set_env_rust_log_variable_if_not_defined_by_env;

#[tokio::main]
async fn main() -> Result<()> {
    set_env_rust_log_variable_if_not_defined_by_env("info");
    env_logger::init();
    process_main().await
}
