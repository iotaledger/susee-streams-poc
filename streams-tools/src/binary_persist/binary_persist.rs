use std::{
    fmt,
    convert::TryInto,
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


impl BinaryPersist for u64 {
    fn needed_size(&self) -> usize {
        8
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[0..8].copy_from_slice(&self.to_le_bytes());
        Ok(8)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        Ok(u64::from_le_bytes(buffer[0..8].try_into().expect("slice with incorrect length")))
    }
}

impl BinaryPersist for u32 {
    fn needed_size(&self) -> usize {
        4
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[0..4].copy_from_slice(&self.to_le_bytes());
        Ok(4)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        Ok(u32::from_le_bytes(buffer[0..4].try_into().expect("slice with incorrect length")))
    }
}

impl BinaryPersist for u16 {
    fn needed_size(&self) -> usize { 2 }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[0..2].copy_from_slice(&self.to_le_bytes());
        Ok(2)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        Ok(u16::from_le_bytes(buffer[0..2].try_into().expect("slice with incorrect length")))
    }
}

impl BinaryPersist for u8 {
    fn needed_size(&self) -> usize { 1 }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        buffer[0..1].copy_from_slice(&self.to_le_bytes());
        Ok(1)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> {
        Ok(u8::from_le_bytes(buffer[0..1].try_into().expect("slice with incorrect length")))
    }
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
pub struct EnumeratedPersistableInner(pub u8);

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

pub fn serialize_binary_persistable_and_streams_link<T: BinaryPersist>(binary_persistable: T, streams_link: &String, buffer: &mut [u8], range: &mut Range<usize>) -> Result<()> {
    // Serialize persistable_thing to buffer
    range.increment(binary_persistable.needed_size());
    binary_persistable.to_bytes(&mut buffer[range.clone()]).expect("Serializing 'persistable_thing' failed");
    // streams link
    serialize_string(streams_link, buffer, range)?;
    Ok(())
}

pub fn deserialize_enumerated_persistable_arg_with_one_string<T, E>(buffer: &[u8], range: &mut Range<usize> ) -> Result<T>
    where
        T: Sized + EnumeratedPersistableArgs<E> + Default,
        E: EnumeratedPersistable + 'static + std::cmp::PartialEq + std::fmt::Display
{
    // COMMAND type
    range.increment(E::LENGTH_BYTES);
    let enumerated_persistable = EnumeratedPersistableInner::try_from_bytes::<E>(&buffer[range.clone()])?;
    if enumerated_persistable != *T::INSTANCE {
        bail!("Wrong type T for deserializing {} instance. Wrong type is {}.", T::INSTANCE, enumerated_persistable)
    }
    // persisted steams link utf8 string binary
    let link = deserialize_string(buffer, range)?;
    let mut ret_val = T::default();
    ret_val.set_str_arg(link);
    Ok(ret_val)
}

pub fn calc_string_binary_length( str_arg: &String) -> usize {
    str_arg.as_bytes().len() + USIZE_LEN
}


pub fn serialize_string(str: &String, buffer: &mut [u8], range: &mut Range<usize>) -> Result<()> {
    let str_bytes = str.as_bytes();
    // Length of persisted utf8 string binary
    range.increment(USIZE_LEN);
    BinaryPersist::to_bytes(&(str_bytes.len() as u32), &mut buffer[range.clone()]).expect("Serializing 'length of persisted string' failed");
    // persisted string utf8 bytes
    range.increment(str_bytes.len());
    buffer[range.clone()].copy_from_slice(str_bytes);
    Ok(())
}

pub fn deserialize_string(buffer: &[u8], range: &mut Range<usize> ) -> Result<String> {
    // string length
    range.increment(USIZE_LEN);
    let str_len= u32::try_from_bytes(&buffer[range.clone()]).unwrap();
    // utf8 string
    range.increment(str_len as usize);
    Ok(String::from_utf8(buffer[range.clone()].to_vec())?)
}