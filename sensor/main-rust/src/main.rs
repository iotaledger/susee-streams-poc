// #![no_std]
use anyhow::Result;
use sensor_lib::{process_main};

#[tokio::main]
async fn main() -> Result<()> {

    process_main().await
}
