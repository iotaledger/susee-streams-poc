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

use clap::Values;

use streams::{
    Address,
    User,
};

use streams_tools::{
    subscriber_manager::get_public_key_str,
    binary_persist::{
        Subscription,
    }
};

use susee_tools::SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC;

use crate::std::{
    ClientType,
    SubscriberManagerPlainTextWalletHttpClient
};

pub struct SensorManager {}

impl SensorManager {

    pub fn run_sending_message_again_count_down() {
        for s in 0..SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC {
            print!("Sending Message again in {} secs\r", SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC - s);
            stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }

    pub async fn send_bytes_to_subscriber(buffer: &Vec<u8>, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) {
        match subscriber.send_signed_packet(buffer).await {
            Ok(previous_message) => {
                log::info!("Previous message address now is {}\n\n", previous_message.to_string());
            }
            Err(err) => {
                print!("Got Error while sending Message: {}\r", err);
            }
        }
        // safe_user_state is usually called by the drop handler of the subscriber but
        // as this loop runs until the user presses ctr-c we need this to
        // save the user state immediately.
        // A probably safer alternative would be a tokio::signal::ctrl_c() handler but for
        // our test purposes this approach is sufficient and much simpler.
        subscriber.save_client_state_blocking();
    }

    pub async fn send_file_content_as_msg_in_endless_loop(msg_file: &str, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        let f = File::open(msg_file)?;
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        log::info!("[Sensor] Message file '{}' contains {} bytes payload\n", msg_file, buffer.len());

        loop {
            log::info!("Sending message file {}\n", msg_file);
            Self::send_bytes_to_subscriber(&buffer.clone(), subscriber).await;
            Self::run_sending_message_again_count_down();
        }
    }

    pub async fn send_messages_in_endless_loop(files_to_send: Values<'_>, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        for msg_file in files_to_send.clone() {
            if !Path::new(msg_file).exists(){
                panic!("[Sensor] Can not find message file '{}'", msg_file);
            }
        }
        for msg_file in files_to_send {
            log::info!("[Sensor] Will send message file '{}' every {} secs", msg_file, SUSEE_CONST_SEND_MESSAGE_REPETITION_WAIT_SEC);
            Self::send_file_content_as_msg_in_endless_loop(msg_file, subscriber).await?;
        }

        Ok(())
    }

    pub async fn send_random_message_in_endless_loop(msg_size: usize, subscriber: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        loop {
            let mut random_bytes = Vec::<u8>::with_capacity(msg_size);
            random_bytes.resize_with(msg_size, || {rand::random::<u8>()});
            log::info!("Sending random message of size {}. First message bytes are: {}\n",
                     msg_size,
                     hex::encode(&random_bytes[..8])
            );
            Self::send_bytes_to_subscriber(&random_bytes, subscriber).await;
            Self::run_sending_message_again_count_down()
        }
    }

    fn println_subscription_details(
        subscriber: &User<ClientType>,
        subscription_link: &Address,
        comment: &str,
        key_name: &str,
        init_cnt: u8,
    ) -> Result<Subscription> {
        let public_key = get_public_key_str(subscriber);
        log::info!(
            "[Sensor] {}:
             {} Link:     {}
                  Tangle Index:     {:#?}
             User public key: {}
             Initialization count:  {}\n",
            comment,
            key_name,
            subscription_link.to_string(),
            hex::encode(subscription_link.to_msg_index()),
            public_key,
            init_cnt,

        );
        Ok(Subscription {
            subscription_link: subscription_link.to_string(),
            pup_key: public_key,
            initialization_cnt: init_cnt,
        })
    }

    pub fn println_subscriber_status<'a> (subscriber_manager: &SubscriberManagerPlainTextWalletHttpClient) -> Result<(String, Subscription)>
    {
        let mut subscription: Option<Subscription> = None;
        if let Some(subscriber) = &subscriber_manager.user {
            if let Some(subscription_link) = subscriber_manager.subscription_link {
                subscription = Some(
                    Self::println_subscription_details(
                        &subscriber,
                        &subscription_link,
                        "A subscription with the following details has already been created",
                        "Subscription",
                        subscriber_manager.get_initialization_cnt()
                    ).expect("Error on println_subscription_details")
                );
            }
        }
        if subscription.is_none() {
            log::info!("[Sensor] No existing subscription message found.");
        }

        let mut previous_message_link = "---".to_string();
        if let Some(prev_msg_link) = subscriber_manager.prev_msg_link {
            log::info!("[Sensor] The last previously used message link is: {}", prev_msg_link);
            previous_message_link = prev_msg_link.to_string();
        }
        if let Some(subs) = subscription {
            Ok((previous_message_link, subs))
        } else {
            bail!("No existing subscription")
        }
    }

    pub async fn subscribe_to_channel(announcement_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<(String, String, u8)> {
        let ann_address = Address::from_str(&announcement_link_str).map_err(|e|anyhow!(e))?;
        let sub_msg_link = subscriber_mngr.subscribe(ann_address).await?;
        let initialization_cnt = subscriber_mngr.get_initialization_cnt();
        Self::println_subscription_details(
            &subscriber_mngr.user.as_ref().unwrap(),
            &sub_msg_link,
            "A subscription with the following details has been created",
            "Subscription",
            initialization_cnt,
        )?;
        let public_key_str = get_public_key_str(&subscriber_mngr.user.as_ref().unwrap());
        Ok((sub_msg_link.to_string(), public_key_str, initialization_cnt))
    }

    pub async fn register_keyload_msg(keyload_msg_link_str: &str, subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient) -> Result<()> {
        let keyload_msg_link = Address::from_str(&keyload_msg_link_str).map_err(|e| anyhow!(e))?;
        subscriber_mngr.register_keyload_msg(&keyload_msg_link).await.expect("[Sensor] Error while registering keyload msg");

        Self::println_subscription_details(
            &subscriber_mngr.user.as_ref().unwrap(),
            &keyload_msg_link,
            "Messages will be send in the branch defined by the following keyload message",
            "Keyload  msg",
            subscriber_mngr.get_initialization_cnt(),
        )?;

        Ok(())
    }

    pub async fn clear_client_state(subscriber_mngr: &mut SubscriberManagerPlainTextWalletHttpClient)  -> Result<()> {
        subscriber_mngr.clear_client_state().await?;
        Ok(())
    }
}