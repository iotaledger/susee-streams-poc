use std::{
    fmt,
    mem::size_of,
    ops::{
        Range,
    },
};

use anyhow::{
    Result,
};

use crate::binary_persist::{
    RangeIterator,
    BinaryPersist,
    serialize_binary_persistable_and_streams_link,
    EnumeratedPersistable,
    EnumeratedPersistableInner,
    EnumeratedPersistableArgs,
    calc_string_binary_length,
    deserialize_enumerated_persistable_arg,
    serialize_string,
    deserialize_string
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Confirmation(EnumeratedPersistableInner);

impl Confirmation {
    pub const NO_CONFIRMATION: Confirmation = Confirmation(EnumeratedPersistableInner(0));
    pub const SUBSCRIPTION: Confirmation = Confirmation(EnumeratedPersistableInner(1));
    pub const KEYLOAD_REGISTRATION: Confirmation = Confirmation(EnumeratedPersistableInner(2));
    pub const CLEAR_CLIENT_STATE: Confirmation = Confirmation(EnumeratedPersistableInner(3));
    pub const SEND_MESSAGES: Confirmation = Confirmation(EnumeratedPersistableInner(4));
}

impl EnumeratedPersistable for Confirmation {
    const LENGTH_BYTES: usize = size_of::<u8>();

    fn as_str(&self) -> &'static str {
        return match self {
            &Confirmation::NO_CONFIRMATION => "NO_CONFIRMATION",             // 0
            &Confirmation::SUBSCRIPTION => "SUBSCRIPTION",                   // 1
            &Confirmation::KEYLOAD_REGISTRATION => "KEYLOAD_REGISTRATION",   // 2
            &Confirmation::CLEAR_CLIENT_STATE => "CLEAR_CLIENT_STATE",       // 3
            &Confirmation::SEND_MESSAGES => "SEND_MESSAGES",                 // 4
            _ => "Unknown Confirmation",
        };
    }

    fn as_u8(&self) -> u8 {
        self.0.0
    }

    fn from(inner: EnumeratedPersistableInner) -> Self {
        Self(inner)
    }
}

impl BinaryPersist for Confirmation {
    fn needed_size(&self) -> usize {
        self.0.needed_size()
    }
    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> { self.0.to_bytes(buffer) }
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized { EnumeratedPersistableInner::try_from_bytes::<Confirmation>(buffer) }
}

impl fmt::Display for Confirmation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

#[derive(Default)]
pub struct Subscription {
    pub subscription_link: String,
    pub pup_key: String,
}

impl EnumeratedPersistableArgs<Confirmation> for Subscription {
    const INSTANCE: &'static Confirmation = &Confirmation::SUBSCRIPTION;

    fn set_str_arg(&mut self, str_arg: String) {
        self.subscription_link = str_arg;
    }
}

impl BinaryPersist for Subscription {
    fn needed_size(&self) -> usize {
        Confirmation::LENGTH_BYTES + calc_string_binary_length(&self.subscription_link) + calc_string_binary_length(&self.pup_key)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.subscription_link, buffer, &mut range)?;
        serialize_string(&self.pup_key, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let mut ret_val = deserialize_enumerated_persistable_arg::<Subscription, Confirmation>(buffer, &mut range)?;
        ret_val.pup_key = deserialize_string(buffer, & mut range)?;
        Ok(ret_val)
    }
}