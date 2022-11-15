use clap::Values;

use streams_tools::{
    subscriber_manager::get_public_key_str,
    binary_persist::Subscription,
};

use susee_tools::SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC;

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
    },
    time::Duration,
    thread,
    io::{
        stdout,
        Write
    }
};

use anyhow::{
    Result,
    bail,
    anyhow,
};

use crate::std::{
    ClientType,
    SubscriberManagerPlainTextWalletHttpClient
};

pub struct SensorManager {}

impl SensorManager {

    pub async fn send_file_content_as_msg(msg_file: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<Address> {
        let f = File::open(msg_file)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        println!("[Sensor] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

        let mut prev_message: Option<Address> = None;
        loop {
            println!("Sending message file {}\n", msg_file);
            if let Ok(previous_message) = subscriber.send_signed_packet(&Bytes(buffer.clone())).await {
                println!("Previous message address now is {}\n\n", previous_message.to_string());
                prev_message = Some(previous_message);
                // safe_user_state is usually called by the drop handler of the subscriber but
                // as this loop runs until the user presses ctr-c we need this to
                // save the user state immediately.
                // A probably safer alternative would be a tokio::signal::ctrl_c() handler but for
                // our test puposes this approach is sufficient and much simpler.
                subscriber.save_user_state();
            } else {
                break;
            }
            for s in 0..SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC {
                print!("Sending Message again in {} secs\r", SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC - s);
                stdout().flush().unwrap();
                thread::sleep(Duration::from_secs(1));
            }
        }

        prev_message.ok_or_else(|| anyhow!("Error on Sending message file"))
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

    fn println_subscription_details(subscriber: &Subscriber<ClientType>, subscription_link: &Address, comment: &str, key_name: &str) -> Result<Subscription> {
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
        Ok(Subscription {
            subscription_link: subscription_link.to_string(),
            pup_key: public_key
        })
    }

    pub fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerPlainTextWalletHttpClient) -> Result<(String, Subscription)>
    {
        let mut subscription: Option<Subscription> = None;
        if let Some(subscriber) = &subscriber_manager.subscriber {
            if let Some(subscription_link) = subscriber_manager.subscription_link {
                subscription = Some(
                    Self::println_subscription_details(&subscriber, &subscription_link,
                        "A subscription with the following details has already been created", "Subscription"
                    ).expect("Error on println_subscription_details")
                );
            }
        }
        if subscription.is_none() {
            println!("[Sensor] No existing subscription message found.");
        }

        let mut previous_message_link = "---".to_string();
        if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
            println!("[Sensor] The last previously used message link is: {}", prev_msg_link);
            previous_message_link = prev_msg_link.to_string();
        }
        if let Some(subs) = subscription {
            Ok((previous_message_link, subs))
        } else {
            bail!("No existing subscription")
        }
    }

    pub async fn subscribe_to_channel(announcement_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<(String, String)> {
        let ann_address = Address::from_str(&announcement_link_str)?;
        let sub_msg_link = subscriber_mngr.subscribe(&ann_address).await?;

        Self::println_subscription_details(
            &subscriber_mngr.subscriber.as_ref().unwrap(),
            &sub_msg_link,
            "A subscription with the following details has been created",
            "Subscription",
        )?;
        let public_key_str = get_public_key_str(&subscriber_mngr.subscriber.as_ref().unwrap());
        Ok((sub_msg_link.to_string(), public_key_str))
    }

    pub async fn register_keyload_msg(keyload_msg_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        let keyload_msg_link = Address::from_str(&keyload_msg_link_str)?;
        subscriber_mngr.register_keyload_msg(&keyload_msg_link).expect("[Sensor] Error while registering keyload msg");

        Self::println_subscription_details(
            &subscriber_mngr.subscriber.as_ref().unwrap(),
            &keyload_msg_link,
            "Messages will be send in the branch defined by the following keyload message",
            "Keyload  msg",
        )?;

        Ok(())
    }

    pub async fn clear_client_state(subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient)  -> Result<()> {
        subscriber_mngr.clear_client_state().await?;
        Ok(())
    }
}