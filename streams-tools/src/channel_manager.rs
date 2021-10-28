use iota_streams::{
    app_channels::api::tangle::{Address, ChannelType, Bytes},
    core::Result,
};

use crate::{
    CaptureClient,
    helpers::*
};


type Author = iota_streams::app_channels::api::tangle::Author<CaptureClient>;

pub struct ChannelManager {
    client: CaptureClient,
    seed: String,
    author: Option<Author>,
    pub announcement_link: Option<Address>,
    pub keyload_link: Option<Address>,
    pub seq_link:  Option<Address>,
    pub prev_msg_link:  Option<Address>,
}

impl ChannelManager {
    pub fn new(node_url: &str) -> Self {
        Self {
            seed: create_seed(),
            client: CaptureClient::new_from_url(node_url),
            author: None,
            announcement_link: None,
            keyload_link: None,
            seq_link: None,
            prev_msg_link: None,
        }
    }

    pub async fn create_announcement(&mut self) -> Result<Address> {
        if self.author.is_some() {
            panic!("This channel already has been announced")
        }
        let mut author = Author::new(
            self.seed.as_str(),
            ChannelType::SingleBranch,
            self.client.clone(),
        );
        let announcement_link = author.send_announce().await?;
        self.author = Some(author);
        self.announcement_link = Some(announcement_link);
        Ok(announcement_link)
    }

    pub async fn add_subscribers(&mut self, subs_addresses: &Vec<&Address>) -> Result<Address> {
        if self.author.is_none() {
            panic!("This channel has not been announced. Use create_announcement() before using this function.")
        }

        let author = self.author.as_mut().unwrap() ;
        for addr in subs_addresses {
            author.receive_subscribe(addr).await?;
        }

        let (keyload_link, _seq) = author.send_keyload_for_everyone(&self.announcement_link.unwrap()).await?;
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
        let (msg_link, _seq_link) = author.send_signed_packet(
            &self.prev_msg_link.as_ref().unwrap(),
            &Bytes::default(),
            input,
        ).await?;
        self.prev_msg_link = Some(msg_link);
        Ok(msg_link)
    }
}