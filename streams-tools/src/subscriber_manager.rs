use iota_streams::{
    app_channels::api::tangle::{
        Address
    },
    core::Result,
};

use crate::{
    CaptureClient,
    helpers::*
};

type Subscriber = iota_streams::app_channels::api::tangle::Subscriber<CaptureClient>;

pub struct SubscriberManager {
    client: CaptureClient,
    seed: String,
    subscriber: Option<Subscriber>,
    subscription_link: Option<Address>,
}

impl SubscriberManager {
    pub fn new(node_url: &str) -> Self {
        Self {
            seed: create_seed(),
            client: CaptureClient::new_from_url(node_url),
            subscriber: None,
            subscription_link: None,
        }
    }

    pub async fn subscribe(&mut self, ann_address: &Address) -> Result<Address> {
        if self.subscriber.is_none() {
            let mut subscriber = Subscriber::new(
                self.seed.as_str(),
                self.client.clone(),
            );
            subscriber.receive_announcement(&ann_address).await?;
            let sub_msg_link = subscriber.send_subscribe(&ann_address).await?;
            self.subscriber = Some(subscriber);
            self.subscription_link = Some(sub_msg_link);
        }
        Ok(self.subscription_link.unwrap())
    }
}