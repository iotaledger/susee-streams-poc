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

#[cfg(feature = "std")]
use std::{
    path::Path,
    ops::Range,
    fs::{
        write,
        read,
    }
};

use anyhow::bail;

use crate::{
    SimpleWallet,
};

#[cfg(feature = "std")]
use crate::{
    PlainTextWallet,
    BinaryPersist,
    binary_persistence::{
        TANGLE_ADDRESS_BYTE_LEN,
        RangeIterator
    }
};

use iota_streams::core::prelude::hex;

#[cfg(feature = "std")]
use futures::executor::block_on;
#[cfg(feature = "std")]
use iota_streams::app::transport::tangle::TangleAddress;


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
        let ret_val = Self {
            wallet,
            serialization_file: serialization_file.clone(),
            client,
            subscriber: None,
            announcement_link: None,
            subscription_link: None,
            prev_msg_link: None,
        };

        // if let Some(serial_file_name) = serialization_file {
        //     if Path::new(serial_file_name.as_str()).exists(){
        //         import_from_serialization_file(serial_file_name.as_str(), &mut ret_val).await
        //             .expect("Try to import Subscriber state from serialization file");
        //     }
        // }

        ret_val
    }

    pub async fn subscribe(&mut self, ann_address: &Address) -> Result<Address> {
        if self.subscriber.is_none() {
            let mut subscriber = Subscriber::new(
                self.wallet.get_seed(),
                self.client.clone(),
            );
            println!("[SubscriberManager] subscriber created");

            subscriber.receive_announcement(&ann_address).await?;
            println!("[SubscriberManager] subscriber announcement received");
            /*
            let sub_msg_link = subscriber.send_subscribe(&ann_address).await?;
            self.announcement_link = subscriber.announcement_link().clone();
            self.subscriber = Some(subscriber);
            self.subscription_link = Some(sub_msg_link);
             */
    } else {
        // TODO: When the subscription link is known after import_from_serialization_file
        //      this fn call can be handled more gracefully here
        bail!("[SubscriberManager.subscribe()] - Already subscribed. announcement_link: {}", self.announcement_link.unwrap())
    }
    Ok(self.subscription_link.unwrap())
}

pub fn register_keyload_msg(&mut self, keyload_address: &Address) -> Result<()> {
/*
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
    println!("[SubscriberManager.subscribe()] - Se keyload message link as new previous message link
                          Keyload message link: {}\n",
             keyload_address.to_string(),
    )
}
*/
    self.prev_msg_link = Some(*keyload_address);

    Ok(())
}

pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
    /*
    if self.subscriber.is_none(){
        panic!("[SubscriberManager.subscribe()] - Before sending messages you need to subscribe to a channel. Use subscribe() and register_keyload_msg() before using this function.")
    }
    if self.prev_msg_link.is_none(){
        panic!("[SubscriberManager.subscribe()] - Before sending messages you need to register a keyload message. Use register_keyload_msg() before using this function.")
    }
    */
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

#[cfg(feature = "std")]
async fn export_to_serialization_file(&mut self, file_name: &str) -> Result<()> {
    if let Some(subscriber) = &self.subscriber {
        let static_sized_buffer_front_length =
              TANGLE_ADDRESS_BYTE_LEN               // PREV_MSG_LINK
            + TANGLE_ADDRESS_BYTE_LEN               // SUBSCRIPTION_LINK
        ;
        let mut buffer: Vec<u8> = vec![0; static_sized_buffer_front_length];

        // PREV_MSG_LINK
        let mut range: Range<usize> = RangeIterator::new(TANGLE_ADDRESS_BYTE_LEN);
        self.persist_optional_tangle_address(&mut buffer, &mut range, self.prev_msg_link);

        // SUBSCRIPTION_LINK
        range.increment(TANGLE_ADDRESS_BYTE_LEN);
        self.persist_optional_tangle_address(&mut buffer, &mut range, self.subscription_link);

        // SUBSCRIBER
        buffer.append(&mut subscriber.export(self.wallet.get_serialization_password()).await?);
        write(file_name, &buffer).expect(format!("[SubscriberManager.subscribe()] - Error while writing Subscriber state file '{}'", file_name).as_str());
    }
    Ok(())
}

#[cfg(feature = "std")]
fn persist_optional_tangle_address(&self, buffer: &mut Vec<u8>, range: &Range<usize>, link_to_persist_opt: Option<Address>) {
    if let Some(link_to_persist) = link_to_persist_opt {
        let _size = link_to_persist.to_bytes(&mut buffer[range.clone()]);
    } else {
        buffer[range.clone()].fill(0);
    }
}
}

#[cfg(feature = "std")]
async fn import_from_serialization_file<ClientT: ClientTTrait, WalletT: SimpleWallet>(
file_name: &str,
ret_val: &mut SubscriberManager<ClientT, WalletT>
) -> Result<()>{
let buffer = read(file_name).expect(format!("[SubscriberManager.subscribe()] - Error while opening channel state file '{}'", file_name).as_str());

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

Ok(())
}

#[cfg(feature = "std")]
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