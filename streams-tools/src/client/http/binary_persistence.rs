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

pub static TANGLE_ADDRESS_BYTE_LEN: usize = APPINST_SIZE + MSGID_SIZE;


// This is a custom binary persistence implementation. Before it can be used on an arbitrary
// combination of communicating systems (e.g. ESP32 talking to AMD64 architecture) it must be tested
// before using it in production.
// Following problems are not solved by the current implementation:
// * Little endian and big endian conflicts
// * Versioning conflicts
//
// TODO: Replace the code in this file by one of the following libs
// * https://github.com/tokio-rs/prost
// * https://users.rust-lang.org/t/comparison-of-way-too-many-rust-asn-1-der-libraries/58683
// * https://github.com/Geal/nom

pub trait RangeIterator<Idx> {
    fn new(first_length: Idx) -> Self;
    fn increment(&mut self, next_length: Idx);
}

impl RangeIterator<usize> for Range<usize> {
    fn new(first_length: usize) -> Self {
        Self {
            start: 0usize,
            end: first_length,
        }
    }
    fn increment(&mut self, next_length: usize) {
        self.start = self.end.clone();
        self.end = self.end.clone() + next_length;
    }
}

// Whenever the size of data is persisted into a binary buffer we will use 4 bytes for the length
// information independent from the usize of the system
static USIZE_LEN: usize = 4;

pub trait BinaryPersist {
    fn needed_size(&self) -> usize;
    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize>;

    // static
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized;
}

impl BinaryPersist for u64 {
    fn needed_size(&self) -> usize {
        8
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[0..8].copy_from_slice(&self.to_le_bytes());
        Ok(8)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // Ok(u64::from_le_bytes(buffer.try_into().expect("slice with incorrect length")))
        Ok(u64::from_le_bytes(buffer[0..8].try_into().expect("slice with incorrect length")))
    }
}

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
        buffer[range.clone()].copy_from_slice(&needed_size.to_le_bytes());
        range.increment(self.as_bytes().len());
        buffer[range.clone()].copy_from_slice(self.to_bytes().as_slice());
        // println!("[BinaryPersist-BinaryBody.to_bytes] buffer: {:02X?}", buffer[range.clone()]);
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        let mut u32buf: [u8;4] = [0;4];
        u32buf.clone_from_slice(&buffer[range.clone()]);
        let body_len = u32::from_le_bytes(u32buf);
        range.increment(body_len as usize);
        Ok(<BinaryBody as From<Vec<u8>>>::from(buffer[range].to_vec()))
    }
}

impl BinaryPersist for TangleMessage {
    fn needed_size(&self) -> usize {
        let link_bytes_len = self.link().needed_size();
        let mut len_bytes = 2 * link_bytes_len;                        // LINK + PREV_LINK
        len_bytes += self.body.needed_size();            // BINARY_BODY
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
        // println!("[BinaryPersist-TangleMessage.to_bytes] buffer: {:02X?}", buffer[range.clone()]);
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
        let link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        pos += link.needed_size();
        // PREV_LINK
        let prev_link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        pos += link.needed_size();
        // BINARY_BODY
        let body = BinaryBody::try_from_bytes(&buffer[pos..]).unwrap();

        // TangleMessage
        Ok(TangleMessage::new(link, prev_link, body))
    }
}