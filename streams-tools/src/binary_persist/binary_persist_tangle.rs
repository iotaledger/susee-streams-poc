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
use std::fmt::{Debug, Formatter};

use anyhow::{
    Result,
    Error
};
use log;

use crate::binary_persist::{
    RangeIterator,
    BinaryPersist,
    USIZE_LEN,
    serialize_string,
    deserialize_string,
};
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
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct TangleAddressCompressed {
    pub msgid: MsgId,
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

impl Default for TangleAddressCompressed {
    fn default() -> Self {
        TangleAddressCompressed {
            msgid: MsgId::from_str("").expect("Error on deserializing MsgId from empty string")
        }
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
#[derive(Eq, PartialEq, Clone, Debug)]
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

impl Default for TangleMessageCompressed {
    fn default() -> Self {
        TangleMessageCompressed {
            dev_eui: vec![],
            link: TangleAddressCompressed::default(),
            body: Default::default(),
        }
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

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum StreamsApiFunction {
    SendCompressedMessage = 3,
    ReceiveCompressedMessageFromAddress = 4,
}

impl fmt::Display for StreamsApiFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl StreamsApiFunction {
    const SELF_LEN: usize = 4;

    pub fn from_u32(value: u32) -> Result<Self> {
        match value {
            3 => Ok(StreamsApiFunction::SendCompressedMessage),
            4 => Ok(StreamsApiFunction::ReceiveCompressedMessageFromAddress),
            _ => panic!("Unknown StreamsApiFunction value: {}", value)
        }
    }
}

impl BinaryPersist for StreamsApiFunction {

    fn needed_size(&self) -> usize {
        Self::SELF_LEN
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize for StreamsApiFunction] This StreamsApiFunction needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        let range: Range<usize> = RangeIterator::new(Self::SELF_LEN);
        let api_function = self.clone() as u32;
        BinaryPersist::to_bytes(&api_function, &mut buffer[range.clone()]).expect("Could not persist api_function");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let range: Range<usize> = RangeIterator::new(Self::SELF_LEN);
        let api_function = <u32 as BinaryPersist>::try_from_bytes(&buffer[range.clone()]).expect("Could not deserialize api_function");
        StreamsApiFunction::from_u32(api_function)
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct StreamsApiRequest {
    pub api_function: StreamsApiFunction,
    pub address: String,
    pub message: TangleMessageCompressed,
}

impl BinaryPersist for StreamsApiRequest {
    fn needed_size(&self) -> usize {
        self.api_function.needed_size() + USIZE_LEN + self.address.len() + USIZE_LEN + self.message.needed_size()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize for StreamsApiRequest] This StreamsApiRequest needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // API_FUNCTION
        let mut range: Range<usize> = RangeIterator::new(self.api_function.needed_size());
        BinaryPersist::to_bytes(&self.api_function, &mut buffer[range.clone()]).expect("Could not persist api_function");
        // ADDRESS
        serialize_string(&self.address, buffer, &mut range)?;
        // MESSAGE
        let message_len = self.message.needed_size() as u32;
        range.increment(USIZE_LEN);
        BinaryPersist::to_bytes(&message_len, &mut buffer[range.clone()]).expect("Could not persist message length");
        range.increment(message_len as usize);
        self.message.to_bytes(&mut buffer[range.clone()]).expect("Could not persist message");
        
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(StreamsApiFunction::SELF_LEN);
        // API_FUNCTION
        let api_function = <StreamsApiFunction as BinaryPersist>::try_from_bytes(&buffer[range.clone()]).expect("Could not deserialize api_function");
        // ADDRESS
        let address = deserialize_string(buffer, &mut range).expect("Could not deserialize address");
        // MESSAGE
        range.increment(USIZE_LEN);
        let message_size = u32::try_from_bytes(&buffer[range.clone()]).expect("Could not deserialize message length");
        range.increment(message_size as usize);
        let message = <TangleMessageCompressed as BinaryPersist>::try_from_bytes(&buffer[range.clone()]).expect("Could not deserialize message");

        Ok(StreamsApiRequest {
            api_function,
            address,
            message,
        })
    }
}

impl fmt::Display for StreamsApiRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "StreamsApiRequest:
                     api_function: {}
                     address: {}
                     message bytes length: {}
                ", self.api_function, self.address, self.message.needed_size())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streams_api_function() {
        let api_function = StreamsApiFunction::SendCompressedMessage;
        let mut buffer: Vec<u8> = vec![0; api_function.needed_size()];
        api_function.to_bytes(&mut buffer).expect("Could not serialize api_function");
        let api_function2 = StreamsApiFunction::try_from_bytes(&buffer).expect("Could not deserialize api_function");
        assert_eq!(api_function, api_function2);
    }

    #[test]
    fn test_streams_api_request() {
        let message = TangleMessageCompressed {
            dev_eui: vec![], // Currently the dev_eui is not persisted. See TangleMessageCompressed::to_bytes() for more info
            link: TangleAddressCompressed {
                msgid: MsgId::from_str("f2fceded12d9c7363e0ae9db").expect("Could not build MsgId from string"),
            },
            body: BinaryBody::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        };
        let api_request = StreamsApiRequest {
            api_function: StreamsApiFunction::SendCompressedMessage,
            address: "address".to_string(),
            message,
        };
        let mut buffer: Vec<u8> = vec![0; api_request.needed_size()];
        api_request.to_bytes(&mut buffer).expect("Could not serialize api_request");
        let api_request2 = StreamsApiRequest::try_from_bytes(&buffer).expect("Could not deserialize api_request");
        assert_eq!(api_request, api_request2);
    }
}