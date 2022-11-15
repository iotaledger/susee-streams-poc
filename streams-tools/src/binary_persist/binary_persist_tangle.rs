use iota_streams::app::{
    transport::tangle::{
        TangleMessage,
        TangleAddress,
        APPINST_SIZE,
        MSGID_SIZE,
    },
    message::{
        LinkedMessage,
        HasLink,
        BinaryBody
    }
};

use std::{
    convert::TryInto,
    ops::Range,
};

use anyhow::Result;
use log;

use crate::binary_persist::{
    RangeIterator,
    BinaryPersist,
    USIZE_LEN,
};

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