pub mod helpers;
pub mod channel_manager;
pub mod subscriber_manager;
pub mod capture_client;
pub mod file_stream_client;
pub mod plain_text_wallet;

pub use {
    channel_manager::{
        ChannelManager,
        ChannelManagerPlainTextWallet,
        Author,
    },
    subscriber_manager::SubscriberManager,
    capture_client::CaptureClient,
    file_stream_client::FileStreamClient,
    plain_text_wallet::{
        PlainTextWallet,
        SimpleWallet,
    },
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
