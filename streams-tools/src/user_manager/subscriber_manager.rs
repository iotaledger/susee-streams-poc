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

use std::{
    path::Path,
    ops::Range,
    fs::{
        write,
        read,
        remove_file,
    }
};

use anyhow::bail;
use log;

use crate::{
    SimpleWallet,
    binary_persist::{
        BinaryPersist,
        RangeIterator,
    },
    binary_persist_tangle::{
        TANGLE_ADDRESS_BYTE_LEN,
    }
};

#[cfg(feature = "std")]
use crate::{
    PlainTextWallet,
};

use iota_streams::{
    core::prelude::hex,
    app::transport::tangle::TangleAddress,
};

#[cfg(feature = "std")]
use futures::executor::block_on;

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

pub fn get_public_key_str<ClientT: ClientTTrait>(subscriber: &Subscriber<ClientT>) -> String {
    let own_public_key = subscriber.get_public_key();
    let own_public_key_str = hex::encode(own_public_key.to_bytes().as_slice());
    own_public_key_str
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
            log::debug!("[fn new()] serial_file_name: '{}'", serial_file_name);
            let new_path = Path::new(serial_file_name.as_str());
            log::debug!("[fn new()] new_path: '{}'", new_path.display());
            let path_extists = new_path.exists();
            log::debug!("[fn new()] path_extists: '{}'", path_extists);
            if path_extists {
                log::debug!("[fn new()] Try to import Subscriber state from serialization file");
                import_from_serialization_file(serial_file_name.as_str(), &mut ret_val).await
                    .expect("Error while importing Subscriber state");
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
            log::debug!("[fn subscribe] subscriber created");

            subscriber.receive_announcement(&ann_address).await?;
            log::debug!("[fn subscribe] announcement received");

            let sub_msg_link = subscriber.send_subscribe(&ann_address).await?;
            self.announcement_link = subscriber.announcement_link().clone();
            self.subscriber = Some(subscriber);
            self.subscription_link = Some(sub_msg_link);
        } else {
        // TODO: When the subscription link is known after import_from_serialization_file
        //      this fn call can be handled more gracefully here
            println!("[SubscriberManager.subscribe()] - This subscriber has already subscribed. announcement_link: {}", self.announcement_link.unwrap());
            bail!("[SubscriberManager.subscribe()] - This subscriber has already subscribed. announcement_link: {}", self.announcement_link.unwrap())
    }
    log::debug!("[fn subscribe] returning subscription_link");
    Ok(self.subscription_link.unwrap())
}

    pub fn register_keyload_msg(&mut self, keyload_address: &Address) -> Result<()> {
        if self.subscriber.is_none(){
            panic!("[SubscriberManager.subscribe()] - Before registering a keyload message you need to subscribe to a channel. Use subscribe() before using this function.")
        }

        if let Some(prev_msg_link) = self.prev_msg_link {
            println!("[SubscriberManager.subscribe()] - Replacing the old previous message link with new keyload message link
                                  Old previous message link: {}
                                  Keyload message link: {}\n",
                   prev_msg_link.to_string(),
                   keyload_address.to_string(),
            )
        } else {
            println!("[SubscriberManager.subscribe()] - Set keyload message link as new previous message link
                                  Keyload message link: {}\n",
                     keyload_address.to_string(),
            )
        }
        self.prev_msg_link = Some(*keyload_address);

        Ok(())
    }

    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        if self.subscriber.is_none(){
            panic!("[SubscriberManager.subscribe()] - Before sending messages you need to subscribe to a channel. Use subscribe() and register_keyload_msg() before using this function.")
        }
        if self.prev_msg_link.is_none(){
            panic!("[SubscriberManager.subscribe()] - Before sending messages you need to register a keyload message. Use register_keyload_msg() before using this function.")
        }
        let subscriber = self.subscriber.as_mut().unwrap() ;
        subscriber.sync_state().await?;
        let (msg_link, _seq_link) = subscriber.send_signed_packet(
            &self.prev_msg_link.as_ref().unwrap(),
            &Bytes::default(),
            input,
        ).await?;
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }

    pub async fn safe_client_status_to_disk(&mut self) -> Result<()> {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            self.export_to_serialization_file(serial_file_name.as_str()).await
        } else {
            bail!("[SubscriberManager.safe_client_status_to_disk()] - You need to specify the serialization_file constructor argument before using this function.");
        }
    }

    pub async fn clear_client_state(&mut self) -> Result<()> {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            log::debug!("[fn clear_client_state] - START");

            if Path::new(serial_file_name.as_str()).exists(){
                println!("[SubscriberManager.clear_client_state()] - Removing file {}", serial_file_name);
                remove_file(serial_file_name)?;
            } else {
                println!("[SubscriberManager.clear_client_state()] - Can not remove file {} cause it does not exist.", serial_file_name);
            }

            log::debug!("[fn clear_client_state] - Setting all links and subscriber to None");
            self.prev_msg_link = None;
            self.subscription_link = None;
            self.subscriber = None;

            log::debug!("[fn clear_client_state] - Ok");
            Ok(())
        } else {
            bail!("[SubscriberManager.clear_client_state()] - You need to specify the serialization_file constructor argument before using this function.");
        }
    }

    async fn export_to_serialization_file(&mut self, file_name: &str) -> Result<()> {
        log::debug!("[fn export_to_serialization_file] - START");
        if let Some(subscriber) = &self.subscriber {
            log::debug!("[fn export_to_serialization_file] - subscriber available");
            let static_sized_buffer_front_length =
                  TANGLE_ADDRESS_BYTE_LEN               // PREV_MSG_LINK
                + TANGLE_ADDRESS_BYTE_LEN               // SUBSCRIPTION_LINK
            ;
            let mut buffer: Vec<u8> = vec![0; static_sized_buffer_front_length];
            log::debug!("[fn export_to_serialization_file] - buffer.len: {}", buffer.len());

            // PREV_MSG_LINK
            let mut range: Range<usize> = RangeIterator::new(TANGLE_ADDRESS_BYTE_LEN);
            log::debug!("[fn export_to_serialization_file] - persist PREV_MSG_LINK");
            self.persist_optional_tangle_address(&mut buffer, &mut range, self.prev_msg_link);

            // SUBSCRIPTION_LINK
            range.increment(TANGLE_ADDRESS_BYTE_LEN);
            log::debug!("[fn export_to_serialization_file] - persist SUBSCRIPTION_LINK");
            self.persist_optional_tangle_address(&mut buffer, &mut range, self.subscription_link);

            // SUBSCRIBER
            log::debug!("[fn export_to_serialization_file] - persist SUBSCRIBER");
            let mut persisted_subscriber = subscriber.export(self.wallet.get_serialization_password()).await?;
            log::debug!("[SubscriberManager.export_to_serialization_file()] - persisted_subscriber length: {}", persisted_subscriber.len());
            buffer.append(&mut persisted_subscriber);
            log::debug!("[fn export_to_serialization_file] - write file '{}'", file_name);
            write(file_name, &buffer).expect(format!("[SubscriberManager.subscribe()] - Error while writing Subscriber state file '{}'", file_name).as_str());
        }
        log::debug!("[fn export_to_serialization_file] - Ok");
        Ok(())
    }

    fn persist_optional_tangle_address(&self, buffer: &mut Vec<u8>, range: &Range<usize>, link_to_persist_opt: Option<Address>) {
        if let Some(link_to_persist) = link_to_persist_opt {
            let _size = link_to_persist.to_bytes(&mut buffer[range.clone()]);
        } else {
            buffer[range.clone()].fill(0);
        }
    }
}

async fn import_from_serialization_file<ClientT: ClientTTrait, WalletT: SimpleWallet>(
    file_name: &str,
    ret_val: &mut SubscriberManager<ClientT, WalletT>
) -> Result<()>{
    log::debug!("[fn import_from_serialization_file] - START");
    let buffer = read(file_name).expect(format!("[SubscriberManager::import_from_serialization_file()] - Error while opening channel state file '{}'", file_name).as_str());
    log::debug!("[fn import_from_serialization_file] - buffer len: {}", buffer.len());

    // PREV_MSG_LINK
    let mut range: Range<usize> = RangeIterator::new(TANGLE_ADDRESS_BYTE_LEN);
    ret_val.prev_msg_link = read_optional_tangle_address_from_bytes(&buffer, &range);

    // SUBSCRIPTION_LINK
    range.increment(TANGLE_ADDRESS_BYTE_LEN);
    ret_val.subscription_link = read_optional_tangle_address_from_bytes(&buffer, &range);

    // SUBSCRIBER
    let subscriber_export_len = buffer.len() - range.end;
    range.increment(subscriber_export_len);
    let subscriber = Subscriber::import(
        &buffer[range],
        ret_val.wallet.get_serialization_password(),
        ret_val.client.clone()
    ).await?;
    if let Some(link) = subscriber.announcement_link() {
        ret_val.announcement_link = Some(link.clone());
    }

/*
    let own_public_key_str = get_public_key_str(&subscriber);

    if let Ok(last_states) = subscriber.fetch_state() {
        let sensor_last_state: Vec<(String, Cursor<Address>)> = last_states
            .into_iter()
            .filter(|state| { state.0 == own_public_key_str} )
            .collect();

        if sensor_last_state.len() != 1 {
            bail!("[SubscriberManager.import_from_serialization_file()] - No prev_msg_link or multiple prev_msg_links found: Cnt: {}", sensor_last_state.len())
        }

        ret_val.prev_msg_link = Some(sensor_last_state[0].1.link);
    }
*/
    ret_val.subscriber = Some(subscriber);

    log::debug!("[fn import_from_serialization_file] - Ok");
    Ok(())
}

fn read_optional_tangle_address_from_bytes(
    buffer: &Vec<u8>,
    range: &Range<usize>
) -> Option<Address>{
    let msg_link_res = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[range.clone()]);
    if let Ok(msg_link) = msg_link_res {
        Some(msg_link)
    } else {
        None
    }
}

#[cfg(feature = "std")]
impl<ClientT: ClientTTrait, WalletT: SimpleWallet> Drop for SubscriberManager<ClientT, WalletT>{
    fn drop(&mut self) {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            block_on(self.export_to_serialization_file(serial_file_name.as_str()))
                .expect("Try to export Subscriber state into serialization file");
        }
    }
}

#[cfg(feature = "std")]
pub type SubscriberManagerPlainTextWallet<ClientT> = SubscriberManager<ClientT, PlainTextWallet>;