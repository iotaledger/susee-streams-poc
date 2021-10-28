pub mod helpers;
pub mod channel_manager;
pub mod subscriber_manager;
pub mod capture_client;

pub use {
    channel_manager::ChannelManager,
    subscriber_manager::SubscriberManager,
    capture_client::CaptureClient,
};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
