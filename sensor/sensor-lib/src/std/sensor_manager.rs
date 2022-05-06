use clap::Values;

use streams_tools::{
    subscriber_manager::get_public_key_str,
};

use iota_streams::app_channels::api::{
    tangle::{
        Address,
        Bytes,
        Subscriber,
    }
};

use core::str::FromStr;

use std::{
    fs::File,
    path::Path,
    io::{
        Read,
        BufReader
    }
};

use anyhow::Result;

use crate::std::{ClientType, SubscriberManagerPlainTextWalletHttpClient};

pub struct SensorManager {}

impl SensorManager {

    async fn send_file_content_as_msg(msg_file: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<Address>{
        let f = File::open(msg_file)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        println!("[Sensor] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

        subscriber.send_signed_packet(&Bytes(buffer.clone())).await
    }

    pub async fn send_messages(files_to_send: Values<'_>, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()>{
        for msg_file in files_to_send.clone() {
            if !Path::new(msg_file).exists(){
                panic!("[Sensor] Can not find message file '{}'", msg_file);
            }
        }
        for msg_file in files_to_send {
            let msg_link = Self::send_file_content_as_msg(msg_file, subscriber).await?;
            println!("[Sensor] Sent msg from file '{}': {}, tangle index: {:#}\n", msg_file, msg_link, msg_link.to_msg_index());
        }

        Ok(())
    }

    fn println_subscription_details(subscriber: &Subscriber<ClientType>, subscription_link: &Address, comment: &str, key_name: &str) {
        let public_key = get_public_key_str(subscriber);
        println!(
            "[Sensor] {}:
             {} Link:     {}
                  Tangle Index:     {:#}
             Subscriber public key: {}\n",
            comment,
            key_name,
            subscription_link.to_string(),
            subscription_link.to_msg_index(),
            public_key,
        );
    }

    pub fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerPlainTextWalletHttpClient)
    {
        let mut subscription_exists = false;
        if let Some(subscriber) = &subscriber_manager.subscriber {
            if let Some(subscription_link) = subscriber_manager.subscription_link {
                Self::println_subscription_details(&subscriber, &subscription_link, "A subscription with the following details has already been created", "Subscription");
            }
            subscription_exists = true;
        }
        if !subscription_exists {
            println!("[Sensor] No existing subscription message found.");
        }

        if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
            println!("[Sensor] The last previously used message link is: {}", prev_msg_link);
        }
    }

    pub async fn subscribe_to_channel(announcement_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        let ann_address = Address::from_str(&announcement_link_str)?;
        let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await?;

        Self::println_subscription_details(
            &subscriber_mngr.subscriber.as_ref().unwrap(),
            &sub_msg_link,
            "A subscription with the following details has been created",
            "Subscription",
        );

        Ok(())
    }

    pub async fn register_keyload_msg(keyload_msg_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        let keyload_msg_link = Address::from_str(&keyload_msg_link_str)?;
        subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[Sensor] Error while registering keyload msg");

        Self::println_subscription_details(
            &subscriber_mngr.subscriber.as_ref().unwrap(),
            &keyload_msg_link,
            "Messages will be send in the branch defined by the following keyload message",
            "Keyload  msg",
        );

        Ok(())
    }

    pub async fn clear_client_state(subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient)  -> Result<()> {
        subscriber_mngr.clear_client_state().await?;
        Ok(())
    }
}