use iota_streams::{
    app_channels::api::tangle::{
        Address,
        Message,
    },
    core::Result,
    app::transport::{
        Transport,
        TransportDetails,
        TransportOptions,
    }
};

use crate::{
    file_stream_client::WrappedClient,
    plain_text_wallet::create_seed,
};

type Subscriber<ClientT> = iota_streams::app_channels::api::tangle::Subscriber<ClientT>;

pub struct SubscriberManager<ClientT> {
    client: ClientT,
    seed: String,
    subscriber: Option<Subscriber<ClientT>>,
    subscription_link: Option<Address>,
}

impl<ClientT> SubscriberManager<ClientT>
    where
        ClientT: WrappedClient + Clone + TransportOptions + Transport<Address, Message> + TransportDetails<Address>
{
    pub fn new(node_url: &str) -> Self {
        Self {
            seed: create_seed(),
            client: ClientT::new_from_url(node_url),
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