use iota_streams::{
    app_channels::api::tangle::{
        Address,
        ChannelType,
        Bytes,
    },
    core::Result,
    app::transport::tangle::client::Client
};

use crate::{
    plain_text_wallet::PlainTextWallet,
    SimpleWallet
};

use std::{
    path::Path,
    fs::{
        write,
        read,
    }
};

use iota_streams::app::futures::executor::block_on;
use iota_streams::app_channels::api::tangle::PublicKey;
use iota_streams::app::identifier::Identifier;

pub type Author = iota_streams::app_channels::api::tangle::Author<Client>;

pub struct SubscriberData<'a> {
    pub subscription_link: &'a Address,
    pub public_key: &'a [u8]
}

pub struct ChannelManager<WalletT: SimpleWallet> {
    client: Client,
    wallet: WalletT,
    serialization_file: Option<String>,
    pub author: Option<Author>,
    pub announcement_link: Option<Address>,
    pub keyload_link: Option<Address>,
    pub seq_link:  Option<Address>,
    pub prev_msg_link:  Option<Address>,
}

async fn import_from_serialization_file<WalletT: SimpleWallet>(file_name: &str, ret_val: &mut ChannelManager<WalletT>) -> Result<()> {
    let buffer = read(file_name).expect(format!("Try to open channel state file '{}'", file_name).as_str());
    let author = Author::import(
        &buffer,
        ret_val.wallet.get_serialization_password(),
        ret_val.client.clone()
    ).await?;
    if let Some(link) = author.announcement_link() {
        ret_val.announcement_link = Some(link.clone());
    }
    ret_val.author = Some(author);

    Ok(())
}

impl<WalletT: SimpleWallet> ChannelManager<WalletT> {
    // TOGO CGE: This async new fn should be rewritten as synchronous normal new function.
    //           Problem: Usage of block_on() here results in panic because of the usage of tokio.
    pub async fn new(node_url: &str, wallet: WalletT, serialization_file: Option<String>) -> Self {
        let mut ret_val = Self {
            wallet,
            serialization_file: serialization_file.clone(),
            client: Client::new_from_url(node_url),
            author: None,
            announcement_link: None,
            keyload_link: None,
            seq_link: None,
            prev_msg_link: None,
        };

        if let Some(serial_file_name) = serialization_file {
            if Path::new(serial_file_name.as_str()).exists(){
                import_from_serialization_file(serial_file_name.as_str(), &mut ret_val).await
                    .expect("Try to import Author state from serialization file");
            }
        }

        ret_val
    }

    pub async fn create_announcement(&mut self) -> Result<Address> {
        if self.author.is_some() {
            panic!("This channel already has been announced")
        }
        let mut author = Author::new(
            self.wallet.get_seed(),
            ChannelType::SingleBranch,
            self.client.clone(),
        );
        let announcement_link = author.send_announce().await?;
        self.author = Some(author);
        self.announcement_link = Some(announcement_link);
        Ok(announcement_link)
    }

    pub async fn add_subscribers<'a>(&mut self, subscriber_data: &Vec<SubscriberData<'a>>) -> Result<Address> {
        if self.author.is_none() {
            panic!("This channel has not been announced. Use create_announcement() before using this function.")
        }

        let author = self.author.as_mut().unwrap() ;
        for sub_data in subscriber_data {
            author.receive_subscribe(sub_data.subscription_link).await?;
        }

        let keys: Vec<Identifier> = subscriber_data
            .into_iter()
            .map(|sub_data| {
                PublicKey::from_bytes(sub_data.public_key).unwrap().into()
            })
            .collect();

        let (keyload_link, _seq) = author.send_keyload(
            &self.announcement_link.unwrap(),
            &keys,
        ).await?;

        self.keyload_link = Some(keyload_link);
        self.prev_msg_link = Some(keyload_link);
        self.seq_link = _seq;
        Ok(keyload_link)
    }

    pub async fn send_signed_packet(&mut self, input: &Bytes) -> Result<Address> {
        if self.author.is_none() | self.prev_msg_link.is_none(){
            panic!("This channel has not been announced or no subscribers have been added. Use create_announcement() and add_subscribers() before using this function.")
        }

        let author = self.author.as_mut().unwrap() ;
        author.sync_state().await;
        let (msg_link, _seq_link) = author.send_signed_packet(
            &self.prev_msg_link.as_ref().unwrap(),
            &Bytes::default(),
            input,
        ).await?;
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }

    async fn export_to_serialization_file(&mut self, file_name: &str) -> Result<()> {
        if let Some(author) = &self.author {
            let buffer = author.export( self.wallet.get_serialization_password()).await?;
            write(file_name, &buffer).expect(format!("Try to write Author state file '{}'", file_name).as_str());
        }
        Ok(())
    }
}

impl<WalletT: SimpleWallet> Drop for ChannelManager<WalletT> {
    fn drop(&mut self) {
        if let Some(serial_file_name) = self.serialization_file.clone() {
            block_on(self.export_to_serialization_file(serial_file_name.as_str()))
                .expect("Try to export Author state into serialization file");
        }
    }
}

pub type ChannelManagerPlainTextWallet = ChannelManager<PlainTextWallet>;