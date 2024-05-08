use std::{
    path::Path,
    fs::{
        write,
        read,
    },
    convert::TryFrom
};

use futures::executor::block_on;

use anyhow::{
    anyhow,
    Result
};

use streams::{
    Address,
    User,
    id::Identifier,
};

use lets::{
    transport::tangle::{
        Client,
    },
    id::{
        Ed25519,
        Permissioned
    }
};

use crypto::{
    signatures::ed25519,
};

use crate::{
    binary_persist::Bytes,
    wallet::plain_text_wallet::PlainTextWallet,
    SimpleWallet,
    helpers::{
        get_channel_id_from_link,
        get_iota_node_url,
        SerializationCallbackRefToClosureString
    },
    STREAMS_TOOLS_CONST_DEFAULT_BASE_BRANCH_TOPIC,
};

use super::{
    message_indexer::{
        MessageIndexer,
        MessageIndexerOptions,
    },
    dao::message::MessageDataStoreOptions
};

pub struct SubscriberData<'a> {
    pub subscription_link: &'a Address,
    pub permissioned_public_key: Permissioned<&'a [u8]>,
}

#[derive(Default, Clone)]
pub struct ChannelManagerOptions {
    pub serialization_file: Option<String>,
    pub user_state: Option<Vec<u8>>,
    // If specified, will be called on drop to serialize the user state
    pub serialize_user_state_callback: Option<SerializationCallbackRefToClosureString>,
    pub message_data_store_for_msg_caching: Option<MessageDataStoreOptions>,
    pub throttle_sleep_time_millisecs: Option<u64>,
}

pub struct ChannelManager<WalletT: SimpleWallet> {
    wallet: WalletT,
    serialization_file: Option<String>,
    serialize_user_state_callback: Option<SerializationCallbackRefToClosureString>,
    base_branch_topic: String,
    options: ChannelManagerOptions,
    pub iota_node: String,
    pub user: Option<User<Client<MessageIndexer>>>,
    pub announcement_link: Option<Address>,
    pub keyload_link: Option<Address>,
    pub prev_msg_link:  Option<Address>,
}

async fn import_from_serialization_file<WalletT: SimpleWallet>(file_name: &str, ret_val: &mut ChannelManager<WalletT>, opt: &ChannelManagerOptions) -> Result<()> {
    let buffer = read(file_name).expect(format!("Try to open channel state file '{}'", file_name).as_str());
    import_from_buffer(&buffer, ret_val, opt).await
}

async fn import_from_buffer<WalletT: SimpleWallet>(buffer: &Vec<u8>, ret_val: &mut ChannelManager<WalletT>, opt: &ChannelManagerOptions) -> Result<()> {
    let indexer_options = create_indexer_options(ret_val, opt);
    let indexer = MessageIndexer::new(indexer_options);
    let user = User::<Client<MessageIndexer>>::restore(
        &buffer,
        ret_val.wallet.get_serialization_password(),
        Client::for_node(
            &get_iota_node_url(ret_val.iota_node.as_str()),
            indexer
        ).await.map_err(|e|anyhow!(e))?
    ).await.map_err(|e|anyhow!(e))?;
    if let Some(link) = user.stream_address().clone() {
        ret_val.announcement_link = Some(link.clone());
    }
    ret_val.user = Some(user);

    Ok(())
}

fn create_indexer_options<WalletT: SimpleWallet>(channel_mngr: &ChannelManager<WalletT>, opt: &ChannelManagerOptions) -> MessageIndexerOptions {
    let mut indexer_options = MessageIndexerOptions::new(channel_mngr.iota_node.clone());
    indexer_options.message_data_store = opt.message_data_store_for_msg_caching.clone();
    indexer_options.throttle_sleep_time_millisecs = opt.throttle_sleep_time_millisecs.clone();
    indexer_options
}

fn ed25519_from_bytes(key_data: &[u8]) -> ed25519::PublicKey {
    ed25519::PublicKey::try_from_bytes(<[u8; 32]>::try_from(key_data).unwrap()).unwrap()
}

impl<WalletT: SimpleWallet> ChannelManager<WalletT> {
    // TOGO CGE: This async new fn should be rewritten as synchronous normal new function.
    //           Problem: Usage of block_on() here results in panic because of the usage of tokio.
    pub async fn new(node_url: &str, wallet: WalletT, options: Option<ChannelManagerOptions>) -> Self {
        let opt = options.unwrap_or_default();
        let mut ret_val = Self {
            options: opt.clone(),
            iota_node: node_url.to_string(),
            wallet,
            base_branch_topic: STREAMS_TOOLS_CONST_DEFAULT_BASE_BRANCH_TOPIC.to_string(),
            serialization_file: opt.serialization_file.clone(),
            serialize_user_state_callback: opt.serialize_user_state_callback.clone(),
            user: None,
            announcement_link: None,
            keyload_link: None,
            prev_msg_link: None,
        };

        if let Some(serial_file_name) = &opt.serialization_file {
            if Path::new(serial_file_name.as_str()).exists(){
                import_from_serialization_file(serial_file_name.as_str(), &mut ret_val, &opt).await
                    .expect("Error on importing User state from serialization file");
            }
        } else if let Some(user_state) = &opt.user_state {
            import_from_buffer(&user_state, &mut ret_val, &opt).await
                .expect("Error on importing User state from binary user_state buffer");
        } else {
            log::warn!("No binary user_state or serial_file_name for the user state provided.\n\
            Will use empty Streams user state.")
        }

        ret_val
    }

    pub async fn create_announcement(&mut self) -> Result<Address> {
        if self.user.is_some() {
            panic!("This channel already has been announced")
        }
        let indexer_options = create_indexer_options(self, &self.options);
        let indexer = MessageIndexer::new(indexer_options);
        let mut user= User::builder()
            .with_identity(Ed25519::from_seed(self.wallet.get_seed()))
            .with_transport(
                Client::for_node(
                    &get_iota_node_url(self.iota_node.as_str()),
                    indexer
                ).await.map_err(|e|anyhow!(e))?
            )
            .build();

        let announcement_link = user.create_stream(self.base_branch_topic.as_str()).await
            .map_err(|e|anyhow!(e))?;
        self.user = Some(user);
        self.announcement_link = Some(announcement_link.address());
        Ok(announcement_link.address())
    }

    pub async fn add_subscribers<'a>(&mut self, subscriber_data: &Vec<SubscriberData<'a>>) -> Result<Address> {
        if self.user.is_none() {
            panic!("This channel has not been announced. Use create_announcement() before using this function.")
        }

        let user = self.user.as_mut().unwrap() ;
        for sub_data in subscriber_data {
            user.receive_message(*sub_data.subscription_link).await.map_err(|e|anyhow!(e))?;
        }

        let subscribers: Vec<Permissioned<Identifier>> = subscriber_data
            .into_iter()
            .map(|sub_data| {
                match sub_data.permissioned_public_key {
                    Permissioned::Read(pk_data) => {
                        Permissioned::<Identifier>::Read(ed25519_from_bytes(pk_data).into())
                    },
                    Permissioned::ReadWrite(pk_data, duration) => {
                        Permissioned::<Identifier>::ReadWrite(ed25519_from_bytes(pk_data).into(), duration)
                    },
                    Permissioned::Admin(pk_data) => {
                        Permissioned::<Identifier>::Admin(ed25519_from_bytes(pk_data).into())
                    }
                }
            })
            .collect();

        let keyload_link = user.send_keyload(
            self.base_branch_topic.as_str(),
            subscribers.iter().map(Permissioned::as_ref),
            [],
        ).await.map_err(|e| anyhow!(e))?;

        self.keyload_link = Some(keyload_link.address());
        self.prev_msg_link = Some(keyload_link.address());
        Ok(keyload_link.address())
    }

    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        if self.user.is_none() | self.prev_msg_link.is_none(){
            panic!("This channel has not been announced or no subscribers have been added. Use create_announcement() and add_subscribers() before using this function.")
        }

        let user = self.user.as_mut().unwrap() ;
        user.sync().await.expect("Could not sync_state");
        let msg_link = user.send_signed_packet(
            self.base_branch_topic.clone(),
            &Bytes::default(),
            input,
        ).await.map_err(|e| anyhow!(e))?;
        self.prev_msg_link = Some(msg_link.address());
        Ok(msg_link.address())
    }

    async fn export_to_serialization_file(&mut self, file_name: &str) -> Result<()> {
        if let Some(user) = self.user.as_mut() {
            let buffer = user.backup( self.wallet.get_serialization_password()).await.map_err(|e| anyhow!(e))?;
            write(file_name, &buffer).expect(format!("Try to write User state file '{}'", file_name).as_str());
        }
        Ok(())
    }

    async fn export_to_serialize_callback(&mut self, serialize_callback: SerializationCallbackRefToClosureString) -> Result<Option<usize>> {
        let mut ret_val = None;
        if let Some(user) = self.user.as_mut() {
            let buffer = user.backup( self.wallet.get_serialization_password()).await.map_err(|e| anyhow!(e))?;
            if let Some(announcement_link) = &self.announcement_link {
                if let Some(channel_id ) = get_channel_id_from_link(announcement_link.to_string().as_str()) {
                    let bytes_serialized = serialize_callback(channel_id.clone(), buffer)
                        .expect(format!(
                            "Error on serializing user state via serialize_user_state_callback for channel {}", channel_id).as_str());
                    ret_val = Some(bytes_serialized);
                }
            }
        }
        Ok(ret_val)
    }
}

impl<WalletT: SimpleWallet> Drop for ChannelManager<WalletT> {
    fn drop(&mut self) {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            block_on(self.export_to_serialization_file(serial_file_name.as_str()))
                .expect("Error on exporting User State into serialization file");
        }
        if let Some(serialize_callback_ref) = self.serialize_user_state_callback.clone() {
            block_on(self.export_to_serialize_callback(serialize_callback_ref))
                .expect("Error on exporting User State into serialization callback function");
        }
    }
}

pub type ChannelManagerPlainTextWallet = ChannelManager<PlainTextWallet>;