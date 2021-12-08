use iota_streams::app::{
    transport::tangle::{
        TangleMessage,
        TangleAddress
    },
    message::{
        LinkedMessage,
        HasLink,
        BinaryBody,
        BinaryMessage,
    }
};

use std::{
    convert::TryInto,
    ops::Range,
};
use anyhow::Result;

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
        Ok(u64::from_le_bytes(buffer.try_into().expect("slice with incorrect length")))
    }
}

impl BinaryPersist for TangleAddress {
    fn needed_size(&self) -> usize {
        HasLink::to_bytes(self).len()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let needed_size = HasLink::to_bytes(self).len();
        buffer[0..needed_size].copy_from_slice(HasLink::to_bytes(self).as_slice());
        Ok(needed_size)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        <TangleAddress as HasLink>::try_from_bytes(buffer)
    }
}

impl<F> BinaryPersist for BinaryBody<F> {
    fn needed_size(&self) -> usize {
        USIZE_LEN + self.bytes.len() // 4 bytes for length + binary data of that length
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let needed_size: u32 = self.bytes.len() as u32;
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        buffer[range.clone()].copy_from_slice(&needed_size.to_le_bytes());
        range.increment(self.bytes.len());
        buffer[range.clone()].copy_from_slice(self.bytes.as_slice());
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        let mut range: Range<usize> = RangeIterator::new(USIZE_LEN);
        let mut u32buf = [0;4];
        u32buf.clone_from_slice(&buffer[range.clone()]);
        let body_len = u32::from_le_bytes(u32buf);
        range.increment(body_len as usize);
        Ok(<BinaryBody<F> as From<Vec<u8>>>::from(buffer[range].to_vec()))
    }
}

impl<F> BinaryPersist for TangleMessage<F> {
    fn needed_size(&self) -> usize {
        let link_bytes_len = self.link().needed_size();
        let mut len_bytes = self.timestamp.needed_size(); // TIMESTAMP
        len_bytes += 2 * link_bytes_len;                        // LINK + PREV_LINK
        len_bytes += self.binary.body.needed_size();            // BINARY_BODY
        len_bytes
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() < self.needed_size() {
            panic!("[BinarySerialize  for TangleMessage] This TangleMessage needs {} bytes but \
                    the provided buffer length is only {} bytes.", self.needed_size(), buffer.len());
        }
        // TIMESTAMP
        let mut range: Range<usize> = RangeIterator::new(self.timestamp.needed_size());
        self.timestamp.to_bytes(&mut buffer[range.clone()]).expect("Could not persist message timestamp");
        // LINK
        let link_bytes_len = self.link().needed_size();
        range.increment(link_bytes_len);
        BinaryPersist::to_bytes(self.link(), &mut buffer[range.clone()]).expect("Could not persist message link");
        // PREV_LINK
        range.increment(link_bytes_len);
        BinaryPersist::to_bytes(self.prev_link(), &mut buffer[range.clone()]).expect("Could not persist message prev_link");
        // BINARY_BODY
        range.increment(self.binary.body.needed_size());
        BinaryPersist::to_bytes(&self.binary.body, &mut buffer[range.clone()]).expect("Could not persist message binary.body");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        // TIMESTAMP
        let mut pos: usize = 0;
        let timestamp = u64::try_from_bytes(buffer).unwrap();
        pos += timestamp.needed_size();
        // LINK
        let link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        pos += link.needed_size();
        // PREV_LINK
        let prev_link = <TangleAddress as BinaryPersist>::try_from_bytes(&buffer[pos..]).unwrap();
        pos += link.needed_size();
        // BINARY_BODY
        let body = BinaryBody::try_from_bytes(&buffer[pos..]).unwrap();

        // TangleMessage
        Ok(TangleMessage::with_timestamp(BinaryMessage::new(link, prev_link, body), timestamp))
    }
}