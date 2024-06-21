use std::{
    ops::Range,
    rc::Rc,
    str::FromStr,
};

use anyhow::{Result, bail, anyhow};

use streams::{
    Address,
    User,
    Error as StreamsError,
    id::Ed25519,
};

use lets::{
    message::TransportMessage,
    transport::Transport,
    id::Identifier
};

use crate::{
    wallet::SimpleWallet,
    binary_persist::{
        Bytes,
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
    StreamsTransport,
    STREAMS_TOOLS_CONST_DEFAULT_BASE_BRANCH_TOPIC,
};

#[cfg(feature = "std")]
use crate::{
    PlainTextWallet,
};

#[cfg(feature = "std")]
use futures::{
    executor::block_on,
    TryStreamExt,
};
#[cfg(feature = "smol_rt")]
use smol::{
    block_on,
    stream::StreamExt
};
use std::cell::RefCell;

pub trait SubscriberPersistence {
    fn is_client_state_existing(&self) -> Result<bool>;
    fn get_latest_client_state(&self) -> Result<Vec<u8>>;
    fn persist_new_client_state(&mut self, client_state: Vec<u8>) -> Result<usize>;
    fn clear_client_state(&mut self) -> Result<()>;
}

pub struct SubscriberManager<TransportT, WalletT: SimpleWallet>
{
    transport: TransportT,
    wallet: WalletT,
    subscriber_persistence: Rc<RefCell<dyn SubscriberPersistence>>,
    compressed: Rc<CompressedStateManager>,
    compressed_subscription_handle: usize,
    is_synced: bool,
    base_branch_topic: String,
    pub user: Option<User<TransportT>>,
    pub announcement_link: Option<Address>,
    pub subscription_link: Option<Address>,
    pub prev_msg_link:  Option<Address>,
}

impl<TransportT, WalletT: SimpleWallet> SubscriberManager<TransportT, WalletT>
where
    TransportT: StreamsTransport,
{
    pub async fn new(mut transport: TransportT, wallet: WalletT, subscriber_persistence: Rc<RefCell<dyn SubscriberPersistence>>) -> Self {
        transport.set_initialization_cnt(wallet.get_initialization_cnt());
        let mut ret_val = Self::create_new_instance(
            transport,
            wallet,
            subscriber_persistence.clone()
        ).await;

        let is_client_state_existing = match subscriber_persistence.borrow().is_client_state_existing(){
            Ok(is_initialized) => is_initialized,
            Err(err) => {
                log::error!("subscriber_persistence.is_client_state_existing() resulted in error: {}", err);
                panic!("subscriber_persistence.is_client_state_existing() resulted in error. See log for details");
            }
        };

        if is_client_state_existing {
            log::debug!("[fn new()] Try to import User state from subscriber_persistence");
            import_from_client_data_persistence(subscriber_persistence, &mut ret_val).await
                .expect("Error while importing User state");
        }
        ret_val
    }

    fn subscribe_compressed_message_state(&mut self) -> Result<()>{
        if self.user.is_none() {
            self.compressed_subscription_handle = self.transport.subscribe_listener(self.compressed.clone())?;
            Ok(())
        } else {
            bail!("You need to subscribe to self.client before self.user is created.\
                   Otherwise the CompressedStateListener will not be cloned into self.user")
        }
    }

    pub async fn clear_client_state(&mut self) -> Result<()> {
        log::debug!("[fn clear_client_state()] START");

        log::debug!("[fn clear_client_state()] Calling subscriber_persistence.clear_client_state()");
        self.subscriber_persistence.borrow_mut().clear_client_state()?;

        log::debug!("[fn clear_client_state()] Setting all links and user to None");
        self.prev_msg_link = None;
        self.subscription_link = None;
        self.user = None;
        self.transport.set_initial_use_compressed_msg_state(false);
        self.transport.remove_listener(self.compressed_subscription_handle);

        log::debug!("[fn clear_client_state()] Ok");
        Ok(())
    }
}

impl<TSR, TransportT, WalletT: SimpleWallet> SubscriberManager<TransportT, WalletT>
    where
        TransportT: StreamsTransport + for <'a> Transport<'a, Msg = TransportMessage, SendResponse = TSR>,
{
    fn create_user(&mut self) -> User<TransportT> {
        self.subscribe_compressed_message_state()
            .expect("Error while doing CompressedStatePublish::subscribe on self.client");
        User::builder()
            .with_identity(Ed25519::from_seed(self.wallet.get_seed()))
            .with_transport(self.transport.clone())
            .lean()
            .is_only_publisher()
            .build()
    }

    pub async fn subscribe(&mut self, ann_address: Address) -> Result<Address> {
        if self.user.is_none() {
            // This SubscriberManager has never subscribed before to a streams channel
            self.subscribe_with_cleared_client_state(ann_address.clone()).await?;
            // We do not increment_initialization_cnt here.
            // The initialization_cnt is only incremented in case of re-initializations
        } else {
            self.subscribe_with_dirty_client_state(ann_address, self.wallet.get_initialization_cnt()).await?
        }

        Ok(self.subscription_link.unwrap())
    }

    async fn subscribe_with_cleared_client_state(&mut self, ann_address: Address) -> Result<()> {
        let mut user = self.create_user();
        log::debug!("[fn subscribe_with_cleared_client_state()] user created");

        user.receive_message(ann_address).await.map_err(|e| anyhow!(e))?;
        log::debug!("[fn subscribe_with_cleared_client_state()] announcement received");

        let sub_msg_link = user.subscribe().await.map_err(|e| anyhow!(e))?;
        self.announcement_link = user.stream_address().clone();
        self.user = Some(user);
        self.subscription_link = Some(sub_msg_link.address());
        Ok(())
    }

    async fn subscribe_with_dirty_client_state(&mut self, ann_address: Address, initialization_cnt: u8) -> Result<()> {
        if initialization_cnt < INITIALIZATION_CNT_MAX_VALUE {
            log::info!("[fn subscribe_with_dirty_client_state()]\n\
                                ------------------------------------------------------------------\n\
                                An already existing subscription will be replaced by a new one.\n\
                                Initialization count will be incremented from {} to {}\n\
                                Maximum Initialization count is {}\n\
                                ------------------------------------------------------------------\n",
                     initialization_cnt,
                     initialization_cnt + 1,
                     INITIALIZATION_CNT_MAX_VALUE,
            );
            self.clear_client_state().await?;
            self.subscribe_with_cleared_client_state(ann_address).await?;
            self.wallet.increment_initialization_cnt()?;

            if initialization_cnt == INITIALIZATION_CNT_MAX_VALUE {
                println_maximum_initialization_cnt_reached_warning("SubscriberManager.subscribe_with_dirty_client_state()", initialization_cnt);
            }
        } else {
            bail!("[SubscriberManager.subscribe_with_dirty_client_state())] Maximum number of channel initializations has been reached. Initialization count is {}",
                    initialization_cnt)
        }
        Ok(())
    }

    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        log::debug!("[fn send_signed_packet()] START");
        if self.user.is_none() {
            panic!("[SubscriberManager.send_signed_packet()] Before sending messages you need to subscribe to a channel. Use subscribe() and register_keyload_msg() before using this function.")
        }
        if self.prev_msg_link.is_none() {
            panic!("[SubscriberManager.send_signed_packet()] Before sending messages you need to register a keyload message. Use register_keyload_msg() before using this function.")
        }
        log::debug!("[fn send_signed_packet()] sync_user_state");
        self.sync_user_state().await?;
        log::debug!("[fn send_signed_packet()] call_user_send_signed_packet");
        let msg_link = self.call_user_send_signed_packet(input).await?;
        log::debug!("[fn send_signed_packet()] set new prev_msg_link");
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }

    async fn call_user_send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        let user = self.user.as_mut().unwrap();
        log::debug!("[fn call_user_send_signed_packet()] user.send_signed_packet()");
        let response = match user.send_signed_packet(
            self.base_branch_topic.clone(),
            &Bytes::default(),
            input,
        ).await
        {
            Ok(response) => response,
            Err(streams_err) => match streams_err {
                StreamsError::MessageMissing(msg_id, _info_str) => {
                    log::debug!("[fn call_user_send_signed_packet()] Got error MessageMissing for MsgId {} - syncing client state", msg_id);
                    user.sync().await.map_err(|e| anyhow!(e))?;
                    self.is_synced = true;
                    log::debug!("[fn call_user_send_signed_packet()] user.send_signed_packet() after MessageLinkNotFoundInStore error");
                    user.send_signed_packet(
                        self.base_branch_topic.clone(),
                        &Bytes::default(),
                        input,
                    ).await.map_err(|e| anyhow!(e))?
                },
                _ => {
                    bail!(streams_err);
                }
            }
        };
        Ok(response.address())
    }

    async fn sync_user_state(&mut self) -> Result<()> {
        let user = self.user.as_mut().unwrap();
        if !self.compressed.get_use_compressed_msg() || !self.is_synced {
            log::debug!("[fn sync_user_state()] syncing client state");
            user.sync().await.map_err(|e| anyhow!(e))?;
            self.is_synced = true;
        }
        Ok(())
    }

    pub async fn register_keyload_msg(&mut self, keyload_address: &Address) -> Result<()> {
        let address_of_fetched_msg: Address;
        if let Some(user) = self.user.as_mut() {
            address_of_fetched_msg = Self::fetch_keyload_from_messages(user, keyload_address).await?;
        } else {
            bail!("[SubscriberManager.register_keyload_msg()] Before registering a keyload message you need to subscribe to a channel. Use subscribe() before using this function.")
        };

        if let Some(prev_msg_link) = self.prev_msg_link {
            log::info!("[fn register_keyload_msg()] Replacing the old previous message link with new keyload message link
                                  Old previous message link: {}
                                  Keyload message link: {}\n",
                     prev_msg_link.to_string(),
                     keyload_address.to_string(),
            )
        } else {
            log::info!("[fn register_keyload_msg()] Set keyload message link as new previous message link
                                  Keyload message link: {}\n",
                     keyload_address.to_string(),
            )
        }

        self.prev_msg_link = Some(address_of_fetched_msg);
        Ok(())
    }

    async fn fetch_keyload_from_messages(user: &mut User<TransportT>, keyload_address: &Address) -> Result<Address> {
        let initial_msg = user
            .messages()
            .try_next()
            .await?
            .ok_or(anyhow!("Did not receive an initial_msg"))?;

        if initial_msg.address != *keyload_address {
            bail!("[SubscriberManager.register_keyload_msg()] Received initial_msg does not match expected keyload_address.\ninitial: {}\nexpexted: {}",
            initial_msg.address, keyload_address);
        }

        let keyload_msg = initial_msg.as_keyload()
            .ok_or(anyhow!("initial_msg is expected to be a keyload msg but it is something else"))?;

        if !keyload_msg.includes_subscriber(user.identifier().unwrap()) {
            bail!("[SubscriberManager.register_keyload_msg()] Received keyload_msg did not include this subscriber.")
        }

        Ok(initial_msg.address)
    }
}

impl<TransportT, WalletT: SimpleWallet> SubscriberManager<TransportT, WalletT>
{
    async fn create_new_instance(transport: TransportT, wallet: WalletT, subscriber_persistence: Rc<RefCell<dyn SubscriberPersistence>>) -> SubscriberManager<TransportT, WalletT> {
        Self {
            wallet,
            is_synced: false,
            subscriber_persistence: subscriber_persistence,
            transport,
            base_branch_topic: STREAMS_TOOLS_CONST_DEFAULT_BASE_BRANCH_TOPIC.to_string(),
            user: None,
            announcement_link: None,
            subscription_link: None,
            prev_msg_link: None,
            compressed: Rc::new(CompressedStateManager::new()),
            compressed_subscription_handle: 0,
        }
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
        let null_address = Address::from_str("00000000000000000000000000000000000000000000000000000000000000000000000000000000:000000000000000000000000")
            .map_err(|e| anyhow!(e))?;
        if let Some(subscription_link) = self.subscription_link {
            if subscription_link != null_address {
                if let Some(prev_msg_link) = self.prev_msg_link {
                    if prev_msg_link != null_address {
                        if subscription_link != prev_msg_link {
                            log::debug!("[fn is_channel_initialized()] subscription_link: {}", subscription_link);
                            log::debug!("[fn is_channel_initialized()] prev_msg_link: {}", prev_msg_link);
                            ret_val = true;
                        } else {
                            log::debug!("[fn is_channel_initialized()] subscription_link == prev_msg_link -> Sensor is not initialized");
                        }
                    } else {
                        log::debug!("[fn is_channel_initialized()] prev_msg_link == null_address -> Sensor is not initialized");
                    }

                } else {
                    log::debug!("[fn is_channel_initialized()] prev_msg_link is None -> Sensor is not initialized");
                }
            } else {
                log::debug!("[fn is_channel_initialized()] subscription_link == null_address -> Sensor is not initialized");
            }

        } else {
            log::debug!("[fn is_channel_initialized()] subscription_link is None -> Sensor is not initialized");
        }

        log::debug!("[fn is_channel_initialized()] returning Ok({})", ret_val);
        Ok(ret_val)
    }

    fn get_serialization_password(&self) -> &str {
        self.wallet.get_serialization_password()
    }

    pub fn get_initialization_cnt(&self) -> u8 {
        self.wallet.get_initialization_cnt()
    }

    async fn export_to_subscriber_persistence(&mut self) -> Result<()> {
        log::debug!("[fn export_to_subscriber_persistence()] START");
        if self.user.is_some() {
            log::debug!("[fn export_to_subscriber_persistence()] user available");
            let static_sized_buffer_front_length =
                TANGLE_ADDRESS_BYTE_LEN               // PREV_MSG_LINK
                    + TANGLE_ADDRESS_BYTE_LEN               // SUBSCRIPTION_LINK
                    + 1                                     // USE_COMPRESSED_MSG
                    + 1                                     // IS_SYNCED
                ;
            let mut buffer: Vec<u8> = vec![0; static_sized_buffer_front_length];
            log::debug!("[fn export_to_subscriber_persistence()] buffer.len: {}", buffer.len());

            // PREV_MSG_LINK
            let mut range: Range<usize> = RangeIterator::new(TANGLE_ADDRESS_BYTE_LEN);
            log::debug!("[fn export_to_subscriber_persistence()] persist PREV_MSG_LINK");
            self.persist_optional_tangle_address(&mut buffer, &mut range, self.prev_msg_link);

            // SUBSCRIPTION_LINK
            range.increment(TANGLE_ADDRESS_BYTE_LEN);
            log::debug!("[fn export_to_subscriber_persistence()] persist SUBSCRIPTION_LINK");
            self.persist_optional_tangle_address(&mut buffer, &mut range, self.subscription_link);

            // USE_COMPRESSED_MSG
            serialize_bool(
                "fn export_to_subscriber_persistence",
                "use_compressed_msg",
                self.compressed.get_use_compressed_msg(),
                &mut buffer,
                &mut range
            );

            // IS_SYNCED
            serialize_bool(
                "fn export_to_subscriber_persistence",
                "is_synced",
                self.is_synced,
                &mut buffer,
                &mut range
            );

            // SUBSCRIBER
            log::debug!("[fn export_to_subscriber_persistence()] persist SUBSCRIBER");
            let passw = self.get_serialization_password().to_string();
            let mut persisted_user= vec![];
            if let Some(user) = self.user.as_mut() {
                persisted_user = user.backup(passw.as_str())
                    .await.map_err(|e| anyhow!(e))?;
                log::debug!("[fn export_to_subscriber_persistence()] persisted_user length: {}", persisted_user.len());
            }
            buffer.append(&mut persisted_user);
            log::debug!("[fn export_to_subscriber_persistence()] persist_latest_client_state to subscriber_persistence");
            self.subscriber_persistence.borrow_mut().persist_new_client_state(buffer)?;
        }
        log::debug!("[fn export_to_subscriber_persistence()] Ok");
        Ok(())
    }

    pub async fn save_client_state(&mut self) -> Result<()> {
        self.export_to_subscriber_persistence().await
    }

    pub fn save_client_state_blocking(&mut self) {
        block_on(self.export_to_subscriber_persistence())
            .expect("Try to export Client state into serialization file");
    }
}

pub fn println_maximum_initialization_cnt_reached_warning(fn_name: &str, current_initialization_cnt: u8) {
    log::info!("\n\n[{}] Warning maximum number of initializations reached:\n\n\
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

async fn import_from_client_data_persistence<'a, TransportT: StreamsTransport, WalletT: SimpleWallet>(
    subscriber_persistence: Rc<RefCell<dyn SubscriberPersistence>>,
    ret_val: &mut SubscriberManager<TransportT, WalletT>
) -> Result<()>{
    log::debug!("[fn import_from_client_data_persistence()] START");
    let buffer = subscriber_persistence
        .borrow()
        .get_latest_client_state()?;
    log::debug!("[fn import_from_client_data_persistence()] buffer len: {}", buffer.len());

    // PREV_MSG_LINK
    let mut range: Range<usize> = RangeIterator::new(TANGLE_ADDRESS_BYTE_LEN);
    ret_val.prev_msg_link = read_optional_tangle_address_from_bytes(&buffer, &range);

    // SUBSCRIPTION_LINK
    range.increment(TANGLE_ADDRESS_BYTE_LEN);
    ret_val.subscription_link = read_optional_tangle_address_from_bytes(&buffer, &range);

    // USE_COMPRESSED_MSG
    let use_compressed_msg = deserialize_bool(
        "fn import_from_client_data_persistence",
        "use_compressed_msg",
        buffer.as_slice(),
        &mut range
    )?;
    ret_val.transport.set_initial_use_compressed_msg_state(use_compressed_msg);
    ret_val.subscribe_compressed_message_state()?;

    // IS_SYNCED
    ret_val.is_synced = deserialize_bool(
        "fn import_from_client_data_persistence",
        "is_synced",
        buffer.as_slice(),
        &mut range
    )?;

    // SUBSCRIBER
    let user_export_len = buffer.len() - range.end;
    range.increment(user_export_len);
    let user = User::restore(
        &buffer[range],
        ret_val.wallet.get_serialization_password(),
        ret_val.transport.clone()
    ).await.map_err(|e| anyhow!(e))?;

    if let Some(address) = user.stream_address() {
        ret_val.announcement_link = Some(address.clone());
    }

    ret_val.user = Some(user);

    log::debug!("[fn import_from_client_data_persistence()] Ok");
    Ok(())
}

fn read_optional_tangle_address_from_bytes(
    buffer: &Vec<u8>,
    range: &Range<usize>
) -> Option<Address>{
    let msg_link_res = <Address as BinaryPersist>::try_from_bytes(&buffer[range.clone()]);
    if let Ok(msg_link) = msg_link_res {
        Some(msg_link)
    } else {
        None
    }
}

pub fn get_public_key_str<'a, TransportT: StreamsTransport>(user: &User<TransportT>) -> String {
    let mut own_public_key_str = "None".to_string();
    if let Some(identifier) = user.identifier() {
        match identifier {
            Identifier::Ed25519(public_key) => {
                own_public_key_str = hex::encode(public_key.to_bytes().as_slice());
            },
            // The following line is commented out because using our Streams feature set Identifier
            // only contains Ed25519. Otherwise the line would result in a "warning: unreachable pattern".
            // _ => {}
        }

    }
    own_public_key_str
}

impl<TransportT, WalletT: SimpleWallet> CompressedStateListen for SubscriberManager<TransportT, WalletT>
{
    fn set_use_compressed_msg(&self, use_compressed_msg: bool) {
        self.compressed.set_use_compressed_msg(use_compressed_msg);
    }

    fn get_use_compressed_msg(&self) -> bool {
        self.compressed.get_use_compressed_msg()
    }
}

#[cfg(feature = "std")]
impl<TransportT, WalletT: SimpleWallet> Drop for SubscriberManager<TransportT, WalletT>
    where
        WalletT: SimpleWallet,
{
    fn drop(&mut self) {
        self.save_client_state_blocking();
    }
}

#[cfg(feature = "std")]
pub type SubscriberManagerPlainTextWallet<TransportT> = SubscriberManager<TransportT, PlainTextWallet>;