use std::convert::{TryInto};

use streams::{
    Address,
};

use lets::{
    message::TransportMessage,
    address::{
        MsgId,
        AppAddr,
    }
};

use std::{
    ops::Range,
    str::{
        FromStr
    },
    fmt,
    fmt::{
        Debug,
        Formatter
    }
};
use std::fmt::Display;

use anyhow::{
    Result,
    Error,
    bail,
    anyhow
};

use log;

use crate::binary_persist::{
    RangeIterator,
    BinaryPersist,
    USIZE_LEN,
    serialize_string,
    deserialize_string,
};

pub const APPADDR_SIZE: usize = 40;
pub const MSGID_SIZE: usize = 12;
pub const TANGLE_ADDRESS_BYTE_LEN: usize = APPADDR_SIZE + MSGID_SIZE;

pub fn as_app_addr(buffer: &[u8]) -> AppAddr {
    let bytes: [u8; APPADDR_SIZE] = buffer[0..APPADDR_SIZE].try_into().expect("slice with incorrect length for AppAddr");
    AppAddr::from(bytes)
}

pub fn as_msg_id(buffer: &[u8]) -> MsgId {
    let bytes: [u8; MSGID_SIZE] = buffer[0..MSGID_SIZE].try_into().expect("slice with incorrect length for MsgId");
    MsgId::from(bytes)
}

impl BinaryPersist for AppAddr {
    fn needed_size(&self) -> usize {
        APPADDR_SIZE
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for AppAddr] This AppAddr needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        buffer[0..self.needed_size()].copy_from_slice(self.as_bytes());
        Ok(self.needed_size())
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        Ok(as_app_addr(buffer))
    }
}

impl BinaryPersist for MsgId {
    fn needed_size(&self) -> usize {
        MSGID_SIZE
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for MsgId] This MsgId needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        buffer[0..self.needed_size()].copy_from_slice(self.as_bytes());
        Ok(self.needed_size())
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        Ok(as_msg_id(buffer))
    }
}

impl BinaryPersist for Address {
    fn needed_size(&self) -> usize { APPADDR_SIZE + MSGID_SIZE }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for Address] This Address needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        let mut range: Range<usize> = RangeIterator::new(self.base().needed_size());
        self.base().to_bytes(&mut buffer[range.clone()]).expect("Serializing appaddr failed");
        range.increment(self.relative().needed_size());
        self.relative().to_bytes(&mut buffer[range.clone()]).expect("Serializing msgid failed");

        Ok(self.needed_size())
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(APPADDR_SIZE);
        let appaddr = AppAddr::try_from_bytes(&buffer[range.clone()]).expect("deserialize appaddr failed");
        range.increment(MSGID_SIZE);
        let msgid = MsgId::try_from_bytes(&buffer[range.clone()]).expect("deserialize msgid failed");
        Ok(Address::new(appaddr, msgid))
    }
}

impl BinaryPersist for TransportMessage {
    fn needed_size(&self) -> usize {
        self.as_ref().len() + USIZE_LEN
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for TransportMessage] This TransportMessage needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // BODY LENGTH
        let bytes_len = self.needed_size() as u32;
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        u32::to_bytes(&bytes_len, &mut buffer[range.clone()]).expect(format!("Could not persist body size").as_str());
        // BODY
        if bytes_len > 0 {
            range.increment(self.as_ref().len());
            buffer[range.clone()].copy_from_slice(self.as_ref());
        }
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // BODY LENGTH
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        let bytes_len = u32::try_from_bytes(&buffer[range.clone()]).unwrap();
        range.increment(bytes_len as usize);
        Ok(TransportMessage::new(buffer[range.clone()].to_vec()))
    }
}

pub struct LinkedMessage<LinkT = Address> {
    pub link: LinkT,
    pub body: TransportMessage,
}

pub fn trans_msg_len(msg: &TransportMessage) -> usize {
    msg.as_ref().len()
}

pub fn trans_msg_encode(msg: &TransportMessage) -> String {
    hex::encode(msg.as_ref())
}

impl<LinkT> LinkedMessage<LinkT> {
    pub fn body_len(&self) -> usize {
        trans_msg_len(&self.body)
    }

    pub fn body_hex_encode(&self) -> String {
        trans_msg_encode(&self.body)
    }
}

impl<LinkT: BinaryPersist + Display> BinaryPersist for LinkedMessage<LinkT> {
    fn needed_size(&self) -> usize {
        let mut len_bytes= self.link.needed_size();     // LINK
        len_bytes += self.body.needed_size();                 // BODY
        len_bytes
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize for LinkedMessage] This LinkedMessage needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // LINK
        let mut range: Range<usize> = RangeIterator::new(self.link.needed_size());
        self.link.to_bytes(&mut buffer[range.clone()]).expect("Could not persist message link");
        log::debug!("[BinaryPersist-LinkedMessage.to_bytes] buffer: {:02X?}", buffer[range.clone()].to_vec());
        // BODY
        range.increment(self.body.needed_size());
        self.body.to_bytes(&mut buffer[range.clone()]).expect("Could not persist message binary.body");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // LINK
        let mut pos: usize = 0;
        log::debug!("[BinaryPersist-LinkedMessage.try_from_bytes] converting LINK");
        let link = LinkT::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-LinkedMessage.try_from_bytes] LINK: {}", link.to_string());
        pos += link.needed_size();

        // BODY
        log::debug!("[BinaryPersist-LinkedMessage.try_from_bytes] converting BODY");
        let body = <TransportMessage as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-LinkedMessage.try_from_bytes] length: {} BODY: {}", trans_msg_len(&body), trans_msg_encode(&body));

        // TangleMessage
        log::debug!("[BinaryPersist-LinkedMessage.try_from_bytes] Ok");
        Ok(LinkedMessage{ link, body })
    }
}

// Replaces Address in case LoraWAN DevEUI is used instead of streams channel ID
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct TangleAddressCompressed {
    pub msgid: MsgId,
    pub initialization_cnt: u8,
}

const INITIALIZATION_CNT_SIZE: usize = 1;
pub const INITIALIZATION_CNT_MAX_VALUE: u8 = u8::MAX;

// Length of a string representation of a TangleAddressCompressed
// MSGID_SIZE * 2                   -> Hex representation of the MsgId
// + 1                              -> ':'
// INITIALIZATION_CNT_SIZE * 2      -> Hex representation of the initialization_cnt
const TANGLE_ADDRESS_COMPRESSED_STR_LENGTH: usize = MSGID_SIZE * 2 + 1 + INITIALIZATION_CNT_SIZE * 2;

impl fmt::Display for TangleAddressCompressed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:02x}", self.msgid, self.initialization_cnt)
    }
}

impl TangleAddressCompressed {
    pub fn from_tangle_address(address: &Address, initialization_cnt: u8) -> Self {
        Self { msgid: address.relative(), initialization_cnt }
    }

    pub fn to_tangle_address(&self, streams_channel_id: &str) -> Result<Address> {
        let app_adr = AppAddr::from_str(streams_channel_id)
            .expect("Error on parsing AppInst from streams_channel_id string");
        Ok(Address::new(app_adr, self.msgid))
    }

    pub fn build_tangle_address_str(msg_id: &str, channel_id: &str) -> String {
        format!("{}:{}", channel_id, msg_id)
    }
}

impl Default for TangleAddressCompressed {
    fn default() -> Self {
        TangleAddressCompressed {
            msgid: MsgId::from_str("000000000000000000000000").expect("Error on deserializing MsgId from empty string"),
            initialization_cnt: 0,
        }
    }
}

impl FromStr for TangleAddressCompressed {
    type Err = Error;

    fn from_str(cmpr_addr_str: &str) -> Result<Self> {
        //
        if cmpr_addr_str.len() != TANGLE_ADDRESS_COMPRESSED_STR_LENGTH {
            bail!("Invalid cmpr_addr_str '{}'. Length needs to be {} but length of given string is {}",
             cmpr_addr_str, TANGLE_ADDRESS_COMPRESSED_STR_LENGTH, cmpr_addr_str.len())
        }
        let msgid_end = MSGID_SIZE * 2;
        let initialization_cnt_start = msgid_end + 1;
        let msgid_str = cmpr_addr_str[..msgid_end].to_string();
        let initialization_cnt_str = cmpr_addr_str[initialization_cnt_start..].to_string();

        let msgid = MsgId::from_str(msgid_str.as_str()).map_err(|e| anyhow!(e))?;
        let initialization_cnt = u8::from_str_radix(&initialization_cnt_str, 16)?;

        Ok(TangleAddressCompressed{msgid, initialization_cnt})
    }
}

impl BinaryPersist for TangleAddressCompressed {
    fn needed_size(&self) -> usize {
        // MSGID_SIZE:  msgid
        // 1:           initialization_cnt
        MSGID_SIZE + 1
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for TangleAddressCompressed] This compressed TangleAddressCompressed needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // MSGID
        let mut range: Range<usize> = RangeIterator::new(MSGID_SIZE);
        buffer[range.clone()].copy_from_slice(self.msgid.as_bytes());
        // INITIALIZATION_CNT
        range.increment(1);
        self.initialization_cnt.to_bytes(&mut buffer[range.clone()]).expect("Error on persisting initialization_cnt");

        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // MSGID
        let mut range: Range<usize> = RangeIterator::new(MSGID_SIZE);
        let msgid = MsgId::try_from_bytes(&buffer[range.clone()]).expect("");
        // INITIALIZATION_CNT
        range.increment(1);
        let initialization_cnt = u8::try_from_bytes(&buffer[range]).expect("Error on reading initialization_cnt");

        Ok(TangleAddressCompressed{msgid, initialization_cnt})
    }
}

// Replaces Message in case LoraWAN DevEUI is used instead of streams channel ID
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct TangleMessageCompressed {
    // Although dev_eui is specified to be a 64bit integer we use Vec<u8> here to be more flexible
    pub dev_eui: Vec<u8>,
    pub link: TangleAddressCompressed,
    pub body: TransportMessage,
}

impl TangleMessageCompressed {
    pub fn from_tangle_message(message: &LinkedMessage, initialization_cnt: u8) -> Self {
        Self {
            // The usage of dev_eui in TangleMessageCompressed is optional.
            dev_eui: Vec::<u8>::new(),
            link: TangleAddressCompressed::from_tangle_address(&message.link, initialization_cnt),
            body: message.body.clone(),
        }
    }

    pub fn to_tangle_message(&self, streams_channel_id: &str) -> Result<LinkedMessage> {
        Ok(LinkedMessage{
            link: self.link.to_tangle_address(streams_channel_id)?,
            body: self.body.clone(),
        })
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
            panic!("[BinarySerialize  for TangleMessageCompressed] This compressed Message needs {} bytes but \
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
        // BODY
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

        // BODY
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] converting BODY");
        let body = TransportMessage::try_from_bytes(&buffer[pos..]).unwrap();
        log::debug!("[BinaryPersist-TangleMessageCompressed.try_from_bytes] length: {} BODY: {}", trans_msg_len(&body), trans_msg_encode(&body));

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
    pub cmpr_address: String,
    pub cmpr_message: TangleMessageCompressed,
}

impl BinaryPersist for StreamsApiRequest {
    fn needed_size(&self) -> usize {
        self.api_function.needed_size() + USIZE_LEN + self.cmpr_address.len() + USIZE_LEN + self.cmpr_message.needed_size()
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
        serialize_string(&self.cmpr_address, buffer, &mut range)?;
        // MESSAGE
        let message_len = self.cmpr_message.needed_size() as u32;
        range.increment(USIZE_LEN);
        BinaryPersist::to_bytes(&message_len, &mut buffer[range.clone()]).expect("Could not persist message length");
        range.increment(message_len as usize);
        self.cmpr_message.to_bytes(&mut buffer[range.clone()]).expect("Could not persist message");
        
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
            cmpr_address: address,
            cmpr_message: message,
        })
    }
}

impl fmt::Display for StreamsApiRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "StreamsApiRequest:
                     api_function: {}
                     address: {}
                     message bytes length: {}
                ", self.api_function, self.cmpr_address, self.cmpr_message.needed_size())
    }
}

// These tests need to be started as follows:
//      > cargo test --package streams-tools --lib binary_persist::binary_persist_tangle::tests
//
#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_persist::test_binary_persistance;

    fn get_compressed_address() -> TangleAddressCompressed{
        TangleAddressCompressed {
            msgid: MsgId::from_str("f2fceded12d9c7363e0ae9db").expect("Could not build MsgId from string"),
            initialization_cnt: 0
        }
    }

    #[test]
    fn test_tangle_address_compressed() {
        test_binary_persistance(get_compressed_address());
    }

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
            link: get_compressed_address(),
            body: BinaryBody::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        };
        let api_request = StreamsApiRequest {
            api_function: StreamsApiFunction::SendCompressedMessage,
            cmpr_address: "address".to_string(),
            cmpr_message: message,
        };
        let mut buffer: Vec<u8> = vec![0; api_request.needed_size()];
        api_request.to_bytes(&mut buffer).expect("Could not serialize api_request");
        let api_request2 = StreamsApiRequest::try_from_bytes(&buffer).expect("Could not deserialize api_request");
        assert_eq!(api_request, api_request2);
    }
}