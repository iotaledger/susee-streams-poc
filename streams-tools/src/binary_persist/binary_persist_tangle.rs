use iota_streams::app::{
    transport::tangle::{
        TangleMessage,
        TangleAddress,
        MsgId,
        APPINST_SIZE,
        MSGID_SIZE,
    },
    message::{
        LinkedMessage,
        HasLink,
        BinaryBody
    }
};

use std::{convert::TryInto, ops::Range, fmt};

use anyhow::{
    Result,
    Error
};
use log;

use crate::binary_persist::{RangeIterator, BinaryPersist, USIZE_LEN};
use iota_streams::app::transport::tangle::AppInst;
use std::str::FromStr;

pub static TANGLE_ADDRESS_BYTE_LEN: usize = APPINST_SIZE + MSGID_SIZE;

impl BinaryPersist for TangleAddress {
    fn needed_size(&self) -> usize { TANGLE_ADDRESS_BYTE_LEN }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let needed_size = HasLink::to_bytes(self).len();
        buffer[0..needed_size].copy_from_slice(HasLink::to_bytes(self).as_slice());
        Ok(needed_size)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        <TangleAddress as HasLink>::try_from_bytes(
            buffer[0..TANGLE_ADDRESS_BYTE_LEN].try_into().expect("slice with incorrect length")
        )
    }
}

impl BinaryPersist for BinaryBody {
    fn needed_size(&self) -> usize {
        USIZE_LEN + self.as_bytes().len() // 4 bytes for length + binary data of that length
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let needed_size: u32 = self.as_bytes().len() as u32;
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        BinaryPersist::to_bytes(&needed_size, &mut buffer[range.clone()]).expect("Serializing needed_size failed");
        range.increment(self.as_bytes().len());
        buffer[range.clone()].copy_from_slice(self.to_bytes().as_slice());
        log::debug!("[BinaryPersist-BinaryBody.to_bytes] buffer: {:02X?}", buffer[range.clone()].to_vec());
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);

        let body_len= u32::try_from_bytes(&buffer[range.clone()]).unwrap();
        log::debug!("[BinaryPersist-BinaryBody.try_from_bytes] body_len: {}", body_len);
        range.increment(body_len as usize);
        log::debug!("[BinaryPersist-BinaryBody.try_from_bytes] Ok");
        Ok(<BinaryBody as From<Vec<u8>>>::from(buffer[range].to_vec()))
    }
}

impl BinaryPersist for TangleMessage {
    fn needed_size(&self) -> usize {
        let link_bytes_len = self.link().needed_size();
        let mut len_bytes = 2 * link_bytes_len;               // LINK + PREV_LINK
        len_bytes += self.body.needed_size();                       // BINARY_BODY
        len_bytes
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for TangleMessage] This TangleMessage needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // LINK
        let link_bytes_len = self.link().needed_size();
        let mut range: Range<usize> = RangeIterator::new(link_bytes_len);
        BinaryPersist::to_bytes(self.link(), &mut buffer[range.clone()]).expect("Could not persist message link");
        log::debug!("[BinaryPersist-TangleMessage.to_bytes] buffer: {:02X?}", buffer[range.clone()].to_vec());
        // PREV_LINK
        range.increment(link_bytes_len);
        BinaryPersist::to_bytes(self.prev_link(), &mut buffer[range.clone()]).expect("Could not persist message prev_link");
        // BINARY_BODY
        range.increment(self.body.needed_size());
        BinaryPersist::to_bytes(&self.body, &mut buffer[range.clone()]).expect("Could not persist message binary.body");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // LINK
        let mut pos: usize = 0;
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] converting LINK");
        let link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] LINK: {}", link.to_string());
        pos += link.needed_size();

        // PREV_LINK
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] converting PREV_LINK");
        let prev_link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] PREV_LINK: {}", prev_link.to_string());
        pos += link.needed_size();
        // BINARY_BODY
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] converting BINARY_BODY");
        let body = BinaryBody::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] length: {} BINARY_BODY: {}", body.to_bytes().len(), body);

        // TangleMessage
        log::debug!("[BinaryPersist-TangleMessage.try_from_bytes] Ok");
        Ok(TangleMessage::new(link, prev_link, body))
    }
}

// Replaces TangleAddress in case LoraWAN DevEUI is used instead of streams channel ID
pub struct TangleAddressCompressed {
    msgid: MsgId,
}

impl fmt::Display for TangleAddressCompressed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msgid)
    }
}

impl TangleAddressCompressed {
    pub fn from_tangle_address(address: &TangleAddress) -> Self {
        Self { msgid: address.msgid }
    }

    pub fn to_tangle_address(&self, streams_channel_id: &str) -> Result<TangleAddress> {
        let app_inst = AppInst::from_str(streams_channel_id)
            .expect("Error on parsing AppInst from streams_channel_id string");
        Ok(TangleAddress::new(app_inst, self.msgid))
    }

    pub fn build_tangle_address_str(msg_id: &str, channel_id: &str) -> String {
        format!("{}:{}", channel_id, msg_id)
    }
}

impl FromStr for TangleAddressCompressed {
    type Err = Error;

    fn from_str(msgid_str: &str) -> Result<Self> {
        let msgid = MsgId::from_str(msgid_str)?;
        Ok(TangleAddressCompressed{msgid})
    }
}

impl BinaryPersist for TangleAddressCompressed {
    fn needed_size(&self) -> usize {
        MSGID_SIZE
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[..MSGID_SIZE].copy_from_slice(self.msgid.as_bytes());
        Ok(MSGID_SIZE)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        Ok(TangleAddressCompressed{
            msgid: MsgId::from(&buffer[..MSGID_SIZE])
        })
    }
}

// Replaces TangleMessage in case LoraWAN DevEUI is used instead of streams channel ID
pub struct TangleMessageCompressed {
    // Although dev_eui is specified to be a 64bit integer we use Vec<u8> here to be more flexible
    pub dev_eui: Vec<u8>,
    pub link: TangleAddressCompressed,
    pub body: BinaryBody,
}

impl TangleMessageCompressed {
    pub fn from_tangle_message(message: &TangleMessage) -> Self {
        Self {
            // The usage of dev_eui in TangleMessageCompressed is optional.
            dev_eui: Vec::<u8>::new(),
            link: TangleAddressCompressed::from_tangle_address(&message.link),
            body: message.body.clone(),
        }
    }

    pub fn to_tangle_message(&self, streams_channel_id: &str) -> Result<TangleMessage> {
        Ok(TangleMessage::new(
            self.link.to_tangle_address(streams_channel_id)?,
            TangleAddress::default(),
            self.body.clone()
        ))
    }
}

// DEV_EUI PERSISTENCE
// We do not persist the dev_eui because it will be communicated by the LoraWAN network
// automatically.
// Therefore all code needed to serialize / deserialize the dev_eui is commented out
impl BinaryPersist for TangleMessageCompressed {
    fn needed_size(&self) -> usize {
        // The dev_eui persistence comment above
        //      let dev_eui_len = self.dev_eui.len() + USIZE_LEN; // + USIZE_LEN because of vec length
        let compressed_link_bytes_len = self.link.needed_size();
        self.body.needed_size() + compressed_link_bytes_len
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for TangleMessageCompressed] This compressed TangleMessage needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }

        // The dev_eui persistence comment above
        //      DEV EUI
        //      let mut range: Range<usize> = RangeIterator::new(0);
        //      serialize_vec_u8("TangleMessageCompressed", "dev_eui", &self.dev_eui, buffer, &mut range);

        // LINK
        let mut range: Range<usize> = RangeIterator::new(self.link.needed_size());
        BinaryPersist::to_bytes(&self.link, &mut buffer[range.clone()]).expect("Could not persist compresssed message link");
        log::debug!("[BinaryPersist-TangleMessageCompressed.to_bytes] buffer: {:02X?}", buffer[range.clone()].to_vec());
        // BINARY_BODY
        range.increment(self.body.needed_size());
        BinaryPersist::to_bytes(&self.body, &mut buffer[range.clone()]).expect("Could not persist message binary.body");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        // The dev_eui persistence comment above
        //      DEV EUI
        //      let mut range: Range<usize> = RangeIterator::new(0);
        //      log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] converting DEV EUI");
        //      let dev_eui = deserialize_vec_u8("TangleMessageCompressed", "dev_eui", &buffer, &mut range);

        // LINK
        let mut pos: usize = 0;
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] converting LINK");
        let link = <TangleAddressCompressed as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] LINK: {}", link.to_string());
        pos += link.needed_size();

        // BINARY_BODY
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] converting BINARY_BODY");
        let body = BinaryBody::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] length: {} BINARY_BODY: {}", body.to_bytes().len(), body);

        // TangleMessageCompressed
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] Ok");
        Ok(TangleMessageCompressed {
            dev_eui: Vec::<u8>::new(),
            link,
            body
        })
    }
}