use streams_tools::binary_persist::{
    Command,
    EnumeratedPersistable,
    BinaryPersist
};

use async_trait::async_trait;

use hyper::{
    Body as HyperBody,
    http::Request as HyperRequest,
};

#[async_trait(?Send)]
pub trait CommandFetcher {
    type Options: Default;
    fn new(options: Option<Self::Options>) -> Self;
    fn get_iota_bridge_url(&self) -> Option<String>;

    async fn fetch_next_command(& self) -> anyhow::Result<(Command, Vec<u8>)>;
    async fn send_confirmation(&self, confirmation_request: HyperRequest<HyperBody>) -> anyhow::Result<()>;
}

#[cfg_attr(feature = "std", allow(dead_code))]
pub (crate) fn deserialize_command(buffer: Vec<u8>) -> anyhow::Result<(Command, Vec<u8>)> {
    let mut ret_val = (Command::NO_COMMAND, Vec::<u8>::default());
    let content_len: usize = buffer.len();
    if content_len >= Command::LENGTH_BYTES {
        log::debug!("[fn deserialize_command()] create Command ret_val. buffer content:\n    length:{}\n    bytes:{:02X?}", content_len, buffer.as_slice());
        let command = Command::try_from_bytes(&buffer[0..Command::LENGTH_BYTES]).unwrap();
        log::debug!("[fn deserialize_command()] return ret_val");
        ret_val = (command, buffer);
    } else {
        log::error!("[fn deserialize_command()] response.content_len() < Command::COMMAND_LENGTH_BYTES");
    }
    Ok(ret_val)
}