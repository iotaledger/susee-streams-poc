use std::{
    fmt,
    ops::Range,
};

use anyhow::{
    Result,
    bail
};
use std::ops::Deref;

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
pub static USIZE_LEN: usize = 4;

pub trait BinaryPersist {
    fn needed_size(&self) -> usize;
    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize>;

    // static
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized;
}

pub trait EnumeratedPersistable {
    const LENGTH_BYTES: usize;

    fn as_str(&self) -> &'static str;

    fn as_u8(&self) -> u8;

    fn from(inner: EnumeratedPersistableInner) -> Self;
}

pub trait EnumeratedPersistableArgs<T: EnumeratedPersistable + 'static> {
    const INSTANCE: &'static T;

    fn set_str_arg(&mut self, str_arg: String);
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EnumeratedPersistableInner( pub u8);

impl EnumeratedPersistableInner {

    pub fn needed_size<T: EnumeratedPersistable>() -> usize {
        T::LENGTH_BYTES as usize
    }

    pub fn to_bytes<T: EnumeratedPersistable>(enum_pers: &T, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() != T::LENGTH_BYTES {
            bail!("Input binary buffer has divergent length of {} bytes. Expected size is {}", buffer.len(), T::LENGTH_BYTES);
        }
        BinaryPersist::to_bytes(&enum_pers.as_u8(), &mut buffer[0..T::LENGTH_BYTES])
    }

    pub fn try_from_bytes<T: EnumeratedPersistable>(buffer: &[u8]) -> Result<T> where T: Sized {
        if buffer.len() < T::LENGTH_BYTES {
            bail!("Input binary buffer to small. length of buffer is {} bytes. Expected size is {} min", buffer.len(), T::LENGTH_BYTES);
        }
        Ok(T::from(EnumeratedPersistableInner(u8::try_from_bytes(&buffer[0..T::LENGTH_BYTES]).unwrap())))
    }

    pub fn fmt<T: EnumeratedPersistable>(enum_pers: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            enum_pers.as_str()
        )
    }
}

impl Deref for EnumeratedPersistableInner {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub fn serialize_persistable_thing_and_streams_link<T: BinaryPersist>(persistable_thing: T, streams_link: &String, buffer: &mut [u8])  -> Result<usize> {
    // Serialize persistable_thing to buffer
    let mut range: Range<usize> = RangeIterator::new(persistable_thing.needed_size());
    persistable_thing.to_bytes(&mut buffer[range.clone()]).expect("Serializing 'persistable_thing' failed");
    let link_bytes = streams_link.as_bytes();
    // Length of persisted link utf8 string binary
    range.increment(USIZE_LEN);
    BinaryPersist::to_bytes(&(link_bytes.len() as u32), &mut buffer[range.clone()]).expect("Serializing 'length of persisted link' failed");
    // persisted link string utf8 bytes
    range.increment(link_bytes.len());
    buffer[range.clone()].copy_from_slice(link_bytes);
    Ok(range.end)
}

pub fn calc_string_binary_length( str_arg: &String) -> usize {
    str_arg.as_bytes().len() + USIZE_LEN
}
