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
    deserialize_enumerated_persistable_arg_with_one_string,
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
    pub const SUBSCRIBER_STATUS: Confirmation = Confirmation(EnumeratedPersistableInner(5));
}

impl EnumeratedPersistable for Confirmation {
    const LENGTH_BYTES: usize = size_of::<u8>();

    fn as_str(&self) -> &'static str {
        return match self {
            &Confirmation::NO_CONFIRMATION => "NO_CONFIRMATION",
            &Confirmation::SUBSCRIPTION => "SUBSCRIPTION",
            &Confirmation::KEYLOAD_REGISTRATION => "KEYLOAD_REGISTRATION",
            &Confirmation::CLEAR_CLIENT_STATE => "CLEAR_CLIENT_STATE",
            &Confirmation::SEND_MESSAGES => "SEND_MESSAGES",
            &Confirmation::SUBSCRIBER_STATUS => "SUBSCRIBER_STATUS",

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
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        EnumeratedPersistableInner::try_from_bytes::<Confirmation>(buffer)
    }
}

impl fmt::Display for Confirmation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) }
}


pub struct Subscription {
    pub subscription_link: String,
    pub pup_key: String,
}

impl Default for Subscription {
    fn default() -> Self {
        Subscription {
            subscription_link: String::from("None"),
            pup_key: String::from("None"),
        }
    }
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
        let mut ret_val = deserialize_enumerated_persistable_arg_with_one_string::<Subscription, Confirmation>(buffer, &mut range)?;
        ret_val.pup_key = deserialize_string(buffer, & mut range)?;
        Ok(ret_val)
    }
}

impl fmt::Display for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Subscription:\n subscription_link: {}\n pup_key: {}", self.subscription_link, self.pup_key)
    }
}

pub struct SubscriberStatus {
    pub previous_message_link: String,
    pub subscription: Subscription,
}

impl Default for SubscriberStatus {
    fn default() -> Self {
        SubscriberStatus {
            previous_message_link: String::from("None"),
            subscription: Subscription::default(),
        }
    }
}

impl EnumeratedPersistableArgs<Confirmation> for SubscriberStatus {
    const INSTANCE: &'static Confirmation = &Confirmation::SUBSCRIBER_STATUS;

    fn set_str_arg(&mut self, str_arg: String) {
        self.previous_message_link = str_arg;
    }
}

impl BinaryPersist for SubscriberStatus {
    fn needed_size(&self) -> usize {
        Confirmation::LENGTH_BYTES + calc_string_binary_length(&self.previous_message_link) + self.subscription.needed_size()
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.previous_message_link, buffer, &mut range)?;
        range.increment(self.subscription.needed_size());
        self.subscription.to_bytes(&mut buffer[range.clone()])?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let mut ret_val = deserialize_enumerated_persistable_arg_with_one_string::<SubscriberStatus, Confirmation>(buffer, &mut range)?;
        ret_val.subscription = Subscription::try_from_bytes(&buffer[range.end..])?;
        Ok(ret_val)
    }
}

impl fmt::Display for SubscriberStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Previously used message link: {}\n{}", self.previous_message_link, self.subscription)
    }
}

#[derive(Default)]
pub struct SendMessages {
    pub previous_message_link: String
}

impl EnumeratedPersistableArgs<Confirmation> for SendMessages {
    const INSTANCE: &'static Confirmation = &Confirmation::SEND_MESSAGES;

    fn set_str_arg(&mut self, str_arg: String) {
        self.previous_message_link = str_arg;
    }
}

impl BinaryPersist for SendMessages {
    fn needed_size(&self) -> usize {
        Confirmation::LENGTH_BYTES + calc_string_binary_length(&self.previous_message_link)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.previous_message_link, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let ret_val = deserialize_enumerated_persistable_arg_with_one_string::<SendMessages, Confirmation>(buffer, &mut range)?;
        Ok(ret_val)
    }
}

impl fmt::Display for SendMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Previous message send: {}", self.previous_message_link)
    }
}


macro_rules! confirmation_without_args {
    ($constant:path, $($name:tt)*) => {
        impl EnumeratedPersistableArgs<Confirmation> for $($name)* {
            const INSTANCE: &'static Confirmation = &$constant;
            fn set_str_arg(&mut self, _str_arg: String) {}
        }

        impl BinaryPersist for $($name)* {
            fn needed_size(&self) -> usize {Self::INSTANCE.needed_size()}

            fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
                Self::INSTANCE.to_bytes(buffer)
            }

            fn try_from_bytes(_buffer: &[u8]) -> Result<Self> where Self: Sized { Ok($($name)*{}) }
        }

        impl fmt::Display for $($name)* {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", Self::INSTANCE)
            }
        }
    }
}

#[derive(Default)]
pub struct KeyloadRegistration {}
confirmation_without_args!(Confirmation::KEYLOAD_REGISTRATION, KeyloadRegistration);

#[derive(Default)]
pub struct ClearClientState {}
confirmation_without_args!(Confirmation::CLEAR_CLIENT_STATE, ClearClientState);
