use iota_streams::{
    app_channels::api::tangle::{
        Address,
        Bytes,
    },
    core::{
        Result,
        Errors as StreamsErrors,
    },
};

use std::{
    path::Path,
    ops::Range,
    rc::Rc,
    option::Option::Some,
    str::FromStr,
    fs::{
        write,
        read,
        remove_file,
    }
};

use anyhow::bail;
use log;

use crate::{
    wallet::SimpleWallet,
    binary_persist::{
        BinaryPersist,
        RangeIterator,
        serialize_bool,
        deserialize_bool,
        binary_persist_tangle::{
            TANGLE_ADDRESS_BYTE_LEN,
        },
        INITIALIZATION_CNT_MAX_VALUE,
    },
    compressed_state::{
        CompressedStateListen,
        CompressedStateManager
    },
    StreamsTransport
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
#[cfg(feature = "smol_rt")]
use smol::block_on;

type Subscriber<ClientT> = iota_streams::app_channels::api::tangle::Subscriber<ClientT>;

pub struct SubscriberManager<TransportT: StreamsTransport, WalletT: SimpleWallet>
{
    transport: TransportT,
    wallet: WalletT,
    serialization_file: Option<String>,
    compressed: Rc<CompressedStateManager>,
    compressed_subscription_handle: usize,
    is_synced: bool,
    pub subscriber: Option<Subscriber<TransportT>>,
    pub announcement_link: Option<Address>,
    pub subscription_link: Option<Address>,
    pub prev_msg_link:  Option<Address>,
}

impl<TransportT: StreamsTransport, WalletT: SimpleWallet> SubscriberManager<TransportT, WalletT>
    where
        TransportT: StreamsTransport
{
    pub async fn new(mut transport: TransportT, wallet: WalletT, serialization_file: Option<String>) -> Self {
        transport.set_initialization_cnt(wallet.get_initialization_cnt());
        let mut ret_val = Self {
            wallet,
            is_synced: false,
            serialization_file: serialization_file.clone(),
            transport,
            subscriber: None,
            announcement_link: None,
            subscription_link: None,
            prev_msg_link: None,
            compressed: Rc::new(CompressedStateManager::new()),
            compressed_subscription_handle: 0,
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

    fn create_subscriber(&mut self) -> Subscriber<TransportT> {
        self.subscribe_compressed_message_state()
            .expect("Error while doing CompressedStatePublish::subscribe on self.client");
        Subscriber::new(
            self.wallet.get_seed(),
            self.transport.clone(),
        )
    }

    pub async fn subscribe(&mut self, ann_address: &Address) -> Result<Address> {
        if self.subscriber.is_none() {
            // This SubscriberManager has never subscribed before to a streams channel
            self.subscribe_with_cleared_client_state(ann_address).await?;
            // We do not increment_initialization_cnt here.
            // The initialization_cnt is only incremented in case of re-initializations
        } else {
            if self.wallet.get_initialization_cnt() < INITIALIZATION_CNT_MAX_VALUE {
                println!("[SubscriberManager.subscribe()]\n\
                                ------------------------------------------------------------------\n\
                                An already existing subscription will be replaced by a new one.\n\
                                Initialization count will be incremented from {} to {}\n\
                                Maximum Initialization count is {}\n\
                                ------------------------------------------------------------------\n",
                         self.wallet.get_initialization_cnt(),
                         self.wallet.get_initialization_cnt() + 1,
                         INITIALIZATION_CNT_MAX_VALUE,
                );
                self.clear_client_state().await?;
                self.subscribe_with_cleared_client_state(ann_address).await?;
                self.wallet.increment_initialization_cnt()?;

                if self.wallet.get_initialization_cnt() == INITIALIZATION_CNT_MAX_VALUE {
                    println_maximum_initialization_cnt_reached_warning("SubscriberManager.subscribe()", self.wallet.get_initialization_cnt());
                }
            }
            else {
                bail!("[SubscriberManager.subscribe())] Maximum number of channel initializations has been reached. Initialization count is {}",
                    self.wallet.get_initialization_cnt())
            }
        }

        Ok(self.subscription_link.unwrap())
    }

    pub fn get_initialization_cnt(&self) -> u8 {
        self.wallet.get_initialization_cnt()
    }

    async fn subscribe_with_cleared_client_state(&mut self, ann_address: &Address) -> Result<()> {
        let mut subscriber = self.create_subscriber();
        log::debug!("[fn subscribe_with_cleared_client_state] subscriber created");

        subscriber.receive_announcement(&ann_address).await?;
        log::debug!("[fn subscribe_with_cleared_client_state] announcement received");

        let sub_msg_link = subscriber.send_subscribe(&ann_address).await?;
        self.announcement_link = subscriber.announcement_link().clone();
        self.subscriber = Some(subscriber);
        self.subscription_link = Some(sub_msg_link);
        Ok(())
    }

    pub fn register_keyload_msg(&mut self, keyload_address: &Address) -> Result<()> {
        if self.subscriber.is_none(){
            panic!("[SubscriberManager.register_keyload_msg()] - Before registering a keyload message you need to subscribe to a channel. Use subscribe() before using this function.")
        }

        if let Some(prev_msg_link) = self.prev_msg_link {
            println!("[SubscriberManager.register_keyload_msg()] - Replacing the old previous message link with new keyload message link
                                  Old previous message link: {}
                                  Keyload message link: {}\n",
                   prev_msg_link.to_string(),
                   keyload_address.to_string(),
            )
        } else {
            println!("[SubscriberManager.register_keyload_msg()] - Set keyload message link as new previous message link
                                  Keyload message link: {}\n",
                     keyload_address.to_string(),
            )
        }
        self.prev_msg_link = Some(*keyload_address);

        Ok(())
    }



    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        log::debug!("[fn send_signed_packet] - START");
        if self.subscriber.is_none(){
            panic!("[SubscriberManager.send_signed_packet()] - Before sending messages you need to subscribe to a channel. Use subscribe() and register_keyload_msg() before using this function.")
        }
        if self.prev_msg_link.is_none(){
            panic!("[SubscriberManager.send_signed_packet()] - Before sending messages you need to register a keyload message. Use register_keyload_msg() before using this function.")
        }
        log::debug!("[fn send_signed_packet] - sync_subscriber_state");
        self.sync_subscriber_state().await?;
        log::debug!("[fn send_signed_packet] - call_subscriber_send_signed_packet");
        let msg_link = self.call_subscriber_send_signed_packet(input).await?;
        log::debug!("[fn send_signed_packet] - set new prev_msg_link");
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }

    async fn sync_subscriber_state(&mut self) -> Result<()>{
        let subscriber = self.subscriber.as_mut().unwrap() ;
        if !self.compressed.get_use_compressed_msg() || !self.is_synced {
            log::debug!("[fn sync_subscriber_state] - syncing client state");
            subscriber.sync_state().await?;
            self.is_synced = true;
        }
        Ok(())
    }

    async fn call_subscriber_send_signed_packet(&mut self, input: &Bytes) -> Result<TangleAddress> {
        let subscriber = self.subscriber.as_mut().unwrap() ;
        log::debug!("[fn call_subscriber_send_signed_packet] - prev_msg_link");
        let prev_msg_link = self.prev_msg_link.as_ref().unwrap();
        log::debug!("[fn call_subscriber_send_signed_packet] - subscriber.send_signed_packet()");
        let (msg_link, _seq_link) = match subscriber.send_signed_packet(
            prev_msg_link,
            &Bytes::default(),
            input,
        ).await
        {
            Ok(msg_link_and_sq_link) => msg_link_and_sq_link,
            Err(err) => match err.downcast_ref() {
                Some(streams_err) => {
                    match streams_err {
                        StreamsErrors::MessageLinkNotFoundInStore(address) => {
                            log::debug!("[fn send_signed_packet] - Got MessageLinkNotFoundInStore error for {} - syncing client state", address);
                            subscriber.sync_state().await?;
                            self.is_synced = true;
                            log::debug!("[fn send_signed_packet] - subscriber.send_signed_packet() after MessageLinkNotFoundInStore error");
                            subscriber.send_signed_packet(
                                prev_msg_link,
                                &Bytes::default(),
                                input,
                            ).await?
                        },
                        _ => {
                            return Err(err);
                        }
                    }
                },
                None => {
                    return Err(err);
                }
            }
        };
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
            self.transport.set_initial_use_compressed_msg_state(false);
            self.transport.remove_listener(self.compressed_subscription_handle);

            log::debug!("[fn clear_client_state] - Ok");
            Ok(())
        } else {
            bail!("[SubscriberManager.clear_client_state()] - You need to specify the serialization_file constructor argument before using this function.");
        }
    }

    fn subscribe_compressed_message_state(&mut self) -> Result<()>{
        if self.subscriber.is_none() {
            self.compressed_subscription_handle = self.transport.subscribe_listener(self.compressed.clone())?;
            Ok(())
        } else {
            bail!("You need to subscribe to self.client before self.subscriber is created.\
                   Otherwise the CompressedStateListener will not be cloned into self.subscriber")
        }
    }

    pub fn save_user_state(&self) {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            block_on(self.export_to_serialization_file(serial_file_name.as_str()))
                .expect("Try to export Subscriber state into serialization file");
        }
    }

    async fn export_to_serialization_file(&self, file_name: &str) -> Result<()> {
        log::debug!("[fn export_to_serialization_file] - START");
        if let Some(subscriber) = &self.subscriber {
            log::debug!("[fn export_to_serialization_file] - subscriber available");
            let static_sized_buffer_front_length =
                  TANGLE_ADDRESS_BYTE_LEN               // PREV_MSG_LINK
                + TANGLE_ADDRESS_BYTE_LEN               // SUBSCRIPTION_LINK
                + 1                                     // USE_COMPRESSED_MSG
                + 1                                     // IS_SYNCED
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

            // USE_COMPRESSED_MSG
            serialize_bool(
                "fn export_to_serialization_file",
                "use_compressed_msg",
                self.compressed.get_use_compressed_msg(),
                &mut buffer,
                &mut range
            );

            // IS_SYNCED
            serialize_bool(
                "fn export_to_serialization_file",
                "is_synced",
                self.is_synced,
                &mut buffer,
                &mut range
            );

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

    pub async fn is_channel_initialized(&self) -> Result<bool> {
        let mut ret_val = false;
        let null_address = Address::from_str("00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000")?;
        if let Some(subscription_link) = self.subscription_link {
            if subscription_link != null_address {
                if let Some(prev_msg_link) = self.prev_msg_link {
                    if prev_msg_link != null_address {
                        if subscription_link != prev_msg_link {
                            log::debug!("[fn - is_channel_initialized()] subscription_link: {}", subscription_link);
                            log::debug!("[fn - is_channel_initialized()] prev_msg_link: {}", prev_msg_link);
                            ret_val = true;
                        } else {
                            log::debug!("[fn - is_channel_initialized()] subscription_link == prev_msg_link -> Sensor is not initialized");
                        }
                    } else {
                        log::debug!("[fn - is_channel_initialized()] prev_msg_link == null_address -> Sensor is not initialized");
                    }

                } else {
                    log::debug!("[fn - is_channel_initialized()] prev_msg_link is None -> Sensor is not initialized");
                }
            } else {
                log::debug!("[fn - is_channel_initialized()] subscription_link == null_address -> Sensor is not initialized");
            }

        } else {
            log::debug!("[fn - is_channel_initialized()] subscription_link is None -> Sensor is not initialized");
        }

        log::debug!("[fn - is_channel_initialized()] returning Ok({})", ret_val);
        Ok(ret_val)
    }
}

pub fn println_maximum_initialization_cnt_reached_warning(fn_name: &str, current_initialization_cnt: u8) {
    println!("\n\n[{}] Warning maximum number of initializations reached:\n\n\
                                ---------------------------------------------------------------\n\
                                ---------------------- W A R N I N G --------------------------\n\
                                ---------------------------------------------------------------\n\
                                The maximum number of initializations has been reached.\n\
                                The initialization count for this sensor now is {}.\n\
                                To reset the initialization count, the flash of the sensor\n\
                                needs to be erased.\n\
                                ---------------------------------------------------------------\n",
             fn_name,
             current_initialization_cnt,
    );
}

async fn import_from_serialization_file<ClientT: StreamsTransport, WalletT: SimpleWallet>(
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

    // USE_COMPRESSED_MSG
    let use_compressed_msg = deserialize_bool(
        "fn import_from_serialization_file",
        "use_compressed_msg",
        buffer.as_slice(),
        &mut range
    )?;
    ret_val.transport.set_initial_use_compressed_msg_state(use_compressed_msg);
    ret_val.subscribe_compressed_message_state()?;

    // IS_SYNCED
    ret_val.is_synced = deserialize_bool(
        "fn import_from_serialization_file",
        "is_synced",
        buffer.as_slice(),
        &mut range
    )?;

    // SUBSCRIBER
    let subscriber_export_len = buffer.len() - range.end;
    range.increment(subscriber_export_len);
    let subscriber = Subscriber::import(
        &buffer[range],
        ret_val.wallet.get_serialization_password(),
        ret_val.transport.clone()
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

pub fn get_public_key_str<ClientT: StreamsTransport>(subscriber: &Subscriber<ClientT>) -> String {
    let own_public_key = subscriber.get_public_key();
    let own_public_key_str = hex::encode(own_public_key.to_bytes().as_slice());
    own_public_key_str
}

impl<ClientT: StreamsTransport, WalletT: SimpleWallet> CompressedStateListen for SubscriberManager<ClientT, WalletT>
    where
        ClientT: StreamsTransport
{
    fn set_use_compressed_msg(&self, use_compressed_msg: bool) {
        self.compressed.set_use_compressed_msg(use_compressed_msg);
    }

    fn get_use_compressed_msg(&self) -> bool {
        self.compressed.get_use_compressed_msg()
    }
}

#[cfg(feature = "std")]
impl<ClientT: StreamsTransport, WalletT: SimpleWallet> Drop for SubscriberManager<ClientT, WalletT>{
    fn drop(&mut self) {
        self.save_user_state();
    }
}

#[cfg(feature = "std")]
pub type SubscriberManagerPlainTextWallet<ClientT> = SubscriberManager<ClientT, PlainTextWallet>;