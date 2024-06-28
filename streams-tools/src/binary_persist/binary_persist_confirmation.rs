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
    serialize_binary_persistable_and_one_string,
    EnumeratedPersistable,
    EnumeratedPersistableInner,
    EnumeratedPersistableArgs,
    calc_string_binary_length,
    deserialize_enumerated_persistable_arg_with_one_string,
    serialize_string,
    deserialize_string
};
use crate::streams_transport::streams_transport::STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Confirmation(EnumeratedPersistableInner);

impl Confirmation {
    pub const NO_CONFIRMATION: Confirmation = Confirmation(EnumeratedPersistableInner(0));
    pub const SUBSCRIPTION: Confirmation = Confirmation(EnumeratedPersistableInner(1));
    pub const KEYLOAD_REGISTRATION: Confirmation = Confirmation(EnumeratedPersistableInner(2));
    pub const CLEAR_CLIENT_STATE: Confirmation = Confirmation(EnumeratedPersistableInner(3));
    pub const SEND_MESSAGES: Confirmation = Confirmation(EnumeratedPersistableInner(4));
    pub const SUBSCRIBER_STATUS: Confirmation = Confirmation(EnumeratedPersistableInner(5));
    pub const DEV_EUI_HANDSHAKE: Confirmation = Confirmation(EnumeratedPersistableInner(6));
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
            &Confirmation::DEV_EUI_HANDSHAKE => "DEV_EUI_HANDSHAKE",

            _ => "Unknown Confirmation",
        };
    }


    fn needs_to_wait_for_tangle_milestone(&self) -> bool {
        return match self {
            &Confirmation::NO_CONFIRMATION => false,
            &Confirmation::SUBSCRIPTION => Subscription::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            &Confirmation::KEYLOAD_REGISTRATION => KeyloadRegistration::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            &Confirmation::CLEAR_CLIENT_STATE => ClearClientState::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            &Confirmation::SEND_MESSAGES => SendMessages::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            &Confirmation::SUBSCRIBER_STATUS => SubscriberStatus::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            &Confirmation::DEV_EUI_HANDSHAKE => DevEuiHandshake::NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE,
            _ => false,
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
    pub initialization_cnt: u8,
}

impl Default for Subscription {
    fn default() -> Self {
        Subscription {
            subscription_link: String::from("None"),
            pup_key: String::from("None"),
            initialization_cnt: 0,
        }
    }
}

impl EnumeratedPersistableArgs<Confirmation> for Subscription {
    const INSTANCE: &'static Confirmation = &Confirmation::SUBSCRIPTION;
    const NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE: bool = true;

    fn set_str_arg(&mut self, str_arg: String) {
        self.subscription_link = str_arg;
    }
}

impl BinaryPersist for Subscription {
    fn needed_size(&self) -> usize {
        let mut ret_val = Confirmation::LENGTH_BYTES;                   // CONFIRMATION_TYPE
        ret_val += calc_string_binary_length(&self.subscription_link); // SUBSCRIPTION_LINK
        ret_val += calc_string_binary_length(&self.pup_key);           // PUP_KEY
        ret_val += 1;                                                        // INITIALIZATION_CNT
        ret_val
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        // CONFIRMATION_TYPE + SUBSCRIPTION_LINK
        serialize_binary_persistable_and_one_string(Self::INSTANCE.clone(), &self.subscription_link, buffer, &mut range)?;
        // PUP_KEY
        serialize_string(&self.pup_key, buffer, &mut range)?;
        // INITIALIZATION_CNT
        range.increment(1);
        self.initialization_cnt.to_bytes(&mut buffer[range.clone()]).expect("Error on persisting initialization_cnt");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        // CONFIRMATION_TYPE + SUBSCRIPTION_LINK
        let mut ret_val = deserialize_enumerated_persistable_arg_with_one_string::<Subscription, Confirmation>(buffer, &mut range)?;
        // PUP_KEY
        ret_val.pup_key = deserialize_string(buffer, & mut range)?;
        // INITIALIZATION_CNT
        range.increment(1);
        ret_val.initialization_cnt = u8::try_from_bytes(&buffer[range]).expect("Error on reading initialization_cnt");

        Ok(ret_val)
    }
}

impl fmt::Display for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Subscription:\n subscription_link: {}\n pup_key: {}\n initialization_cnt: {}",
               self.subscription_link,
               self.pup_key,
               self.initialization_cnt,
        )
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
    const NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE: bool = false;

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
        serialize_binary_persistable_and_one_string(Self::INSTANCE.clone(), &self.previous_message_link, buffer, &mut range)?;
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
    const NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE: bool = false;

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
        serialize_binary_persistable_and_one_string(Self::INSTANCE.clone(), &self.previous_message_link, buffer, &mut range)?;
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
            const NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE: bool = false;

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

pub struct DevEuiHandshake {
    pub dev_eui: String,
}

impl Default for DevEuiHandshake {
    fn default() -> Self {
        DevEuiHandshake {
            dev_eui: STREAMS_TOOLS_CONST_DEV_EUI_NOT_DEFINED.to_string(),
        }
    }
}

impl EnumeratedPersistableArgs<Confirmation> for DevEuiHandshake {
    const INSTANCE: &'static Confirmation = &Confirmation::DEV_EUI_HANDSHAKE;
    const NEEDS_TO_WAIT_FOR_TANGLE_MILESTONE: bool = false;

    fn set_str_arg(&mut self, str_arg: String) {
        self.dev_eui = str_arg;
    }
}

impl BinaryPersist for DevEuiHandshake {
    fn needed_size(&self) -> usize {
        Confirmation::LENGTH_BYTES + calc_string_binary_length(&self.dev_eui)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_one_string(Self::INSTANCE.clone(), &self.dev_eui, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let ret_val = deserialize_enumerated_persistable_arg_with_one_string::<DevEuiHandshake, Confirmation>(buffer, &mut range)?;
        Ok(ret_val)
    }
}

impl fmt::Display for DevEuiHandshake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DevEuiHandshake: dev_eui: {}",
               self.dev_eui,
        )
    }
}
