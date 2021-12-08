use iota_streams::{
    app_channels::api::tangle::{
        Address,
        Message,
        Bytes,
    },
    core::Result,
    app::transport::{
        Transport,
        TransportDetails,
        TransportOptions,
    }
};

use std::path::Path;

use std::fs::{
    write,
    read,
};

use crate::{
    SimpleWallet,
    PlainTextWallet,
};
use iota_streams::app::futures::executor::block_on;

type Subscriber<ClientT> = iota_streams::app_channels::api::tangle::Subscriber<ClientT>;

pub trait ClientTTrait: Clone + TransportOptions + Transport<Address, Message> + TransportDetails<Address> {}
impl<T> ClientTTrait for T where T: Clone + TransportOptions + Transport<Address, Message> + TransportDetails<Address> {}

pub struct SubscriberManager<ClientT: ClientTTrait, WalletT: SimpleWallet>
{
    client: ClientT,
    wallet: WalletT,
    serialization_file: Option<String>,
    pub subscriber: Option<Subscriber<ClientT>>,
    pub announcement_link: Option<Address>,
    pub subscription_link: Option<Address>,
    pub prev_msg_link:  Option<Address>,
}


async fn import_from_serialization_file<ClientT: ClientTTrait, WalletT: SimpleWallet>(
    file_name: &str,
    ret_val: &mut SubscriberManager<ClientT, WalletT>
) -> Result<()>{
    let buffer = read(file_name).expect(format!("Try to open channel state file '{}'", file_name).as_str());
    let subscriber = Subscriber::import(
        &buffer,
        ret_val.wallet.get_serialization_password(),
        ret_val.client.clone()
    ).await?;
    if let Some(link) = subscriber.announcement_link() {
        ret_val.announcement_link = Some(link.clone());
    }
    ret_val.subscriber = Some(subscriber);

    Ok(())
}

impl<ClientT: ClientTTrait, WalletT: SimpleWallet> SubscriberManager<ClientT, WalletT>
    where
        ClientT: ClientTTrait
{
    pub async fn new(client: ClientT, wallet: WalletT, serialization_file: Option<String>) -> Self {
        let mut ret_val = Self {
            wallet,
            serialization_file: serialization_file.clone(),
            client,
            subscriber: None,
            announcement_link: None,
            subscription_link: None,
            prev_msg_link: None,
        };

        if let Some(serial_file_name) = serialization_file {
            if Path::new(serial_file_name.as_str()).exists(){
                import_from_serialization_file(serial_file_name.as_str(), &mut ret_val).await
                    .expect("Try to import Subscriber state from serialization file");
            }
        }

        ret_val
    }

    pub async fn subscribe(&mut self, ann_address: &Address) -> Result<Address> {
        if self.subscriber.is_none() {
            let mut subscriber = Subscriber::new(
                self.wallet.get_seed(),
                self.client.clone(),
            );
            subscriber.receive_announcement(&ann_address).await?;
            let sub_msg_link = subscriber.send_subscribe(&ann_address).await?;
            self.announcement_link = subscriber.announcement_link().clone();
            self.subscriber = Some(subscriber);
            self.subscription_link = Some(sub_msg_link);
        }
        Ok(self.subscription_link.unwrap())
    }

    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        if self.subscriber.is_none() | self.prev_msg_link.is_none(){
            panic!("Before sending messages you need to subscribe to a channel. Use subscribe() before using this function.")
        }
        let subscriber = self.subscriber.as_mut().unwrap() ;
        subscriber.sync_state().await;
        let (msg_link, _seq_link) = subscriber.send_signed_packet(
            &self.prev_msg_link.as_ref().unwrap(),
            &Bytes::default(),
            input,
        ).await?;
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }

    async fn export_to_serialization_file(&mut self, file_name: &str) -> Result<()> {
        if let Some(subscriber) = &self.subscriber {
            let buffer = subscriber.export( self.wallet.get_serialization_password()).await?;
            write(file_name, &buffer).expect(format!("Try to write Subscriber state file '{}'", file_name).as_str());
        }
        Ok(())
    }
}

impl<ClientT: ClientTTrait, WalletT: SimpleWallet> Drop for SubscriberManager<ClientT, WalletT>{
    fn drop(&mut self) {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            block_on(self.export_to_serialization_file(serial_file_name.as_str()))
                .expect("Try to export Subscriber state into serialization file");
        }
    }
}

pub type SubscriberManagerPlainTextWallet<ClientT> = SubscriberManager<ClientT, PlainTextWallet>;