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
    USIZE_LEN,
    serialize_binary_persistable_and_streams_link,
    EnumeratedPersistable,
    EnumeratedPersistableInner,
    EnumeratedPersistableArgs,
    calc_string_binary_length,
    deserialize_enumerated_persistable_arg_with_one_string
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Command(EnumeratedPersistableInner);

impl Command {
    pub const NO_COMMAND: Command = Command(EnumeratedPersistableInner(0));
    pub const START_SENDING_MESSAGES: Command = Command(EnumeratedPersistableInner(1));
    pub const SUBSCRIBE_TO_ANNOUNCEMENT_LINK: Command = Command(EnumeratedPersistableInner(2));
    pub const REGISTER_KEYLOAD_MESSAGE: Command = Command(EnumeratedPersistableInner(3));
    pub const PRINTLN_SUBSCRIBER_STATUS: Command = Command(EnumeratedPersistableInner(4));
    pub const CLEAR_CLIENT_STATE: Command = Command(EnumeratedPersistableInner(5));
}

impl EnumeratedPersistable for Command {
    const LENGTH_BYTES: usize = size_of::<u8>();

    fn as_str(&self) -> &'static str {
        return match self {
            &Command::NO_COMMAND => "NO_COMMAND",
            &Command::START_SENDING_MESSAGES => "START_SENDING_MESSAGES",
            &Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK => "SUBSCRIBE_TO_ANNOUNCEMENT_LINK",
            &Command::REGISTER_KEYLOAD_MESSAGE => "REGISTER_KEYLOAD_Message",
            &Command::PRINTLN_SUBSCRIBER_STATUS => "PRINTLN_SUBSCRIBER_STATUS",
            &Command::CLEAR_CLIENT_STATE => "CLEAR_CLIENT_STATE",
            _ => "Unknown Command",
        };
    }

    fn as_u8(&self) -> u8 {
        self.0.0
    }

    fn from(inner: EnumeratedPersistableInner) -> Self {
        Self(inner)
    }
}

impl BinaryPersist for Command {
    fn needed_size(&self) -> usize {
        self.0.needed_size()
    }
    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> { self.0.to_bytes(buffer) }
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized { EnumeratedPersistableInner::try_from_bytes::<Command>(buffer) }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.as_str()) }
}

#[derive(Default)]
pub struct SubscribeToAnnouncement {
    pub announcement_link: String,
}

impl EnumeratedPersistableArgs<Command> for SubscribeToAnnouncement {
    const INSTANCE: &'static Command = &Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK;

    fn set_str_arg(&mut self, str_arg: String) {
        self.announcement_link = str_arg;
    }
}

impl BinaryPersist for SubscribeToAnnouncement {
    fn needed_size(&self) -> usize {
        Command::LENGTH_BYTES + calc_string_binary_length(&self.announcement_link)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.announcement_link, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let ret_val = deserialize_enumerated_persistable_arg_with_one_string::<SubscribeToAnnouncement, Command>(buffer, &mut range)?;
        Ok(ret_val)
    }
}

#[derive(Default)]
pub struct RegisterKeyloadMessage {
    pub keyload_msg_link: String,
}

impl EnumeratedPersistableArgs<Command> for RegisterKeyloadMessage {
    const INSTANCE: &'static Command = &Command::REGISTER_KEYLOAD_MESSAGE;

    fn set_str_arg(&mut self, str_arg: String) {
        self.keyload_msg_link = str_arg;
    }
}

impl BinaryPersist for RegisterKeyloadMessage {
    fn needed_size(&self) -> usize {
        Command::LENGTH_BYTES + calc_string_binary_length(&self.keyload_msg_link)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.keyload_msg_link, buffer, &mut range)?;
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let mut range: Range<usize> = RangeIterator::new(0);
        let ret_val = deserialize_enumerated_persistable_arg_with_one_string::<RegisterKeyloadMessage, Command>(buffer, &mut range)?;
        Ok(ret_val)
    }
}


#[derive(Default)]
pub struct StartSendingMessages {
    pub wait_seconds_between_repeats: u32,
    pub message_template_key: String,
}

impl EnumeratedPersistableArgs<Command> for StartSendingMessages {
    const INSTANCE: &'static Command = &Command::START_SENDING_MESSAGES;

    fn set_str_arg(&mut self, str_arg: String) {
        self.message_template_key = str_arg;
    }
}

impl BinaryPersist for StartSendingMessages {
    fn needed_size(&self) -> usize {
        Command::LENGTH_BYTES +
            calc_string_binary_length(&self.message_template_key) +     // COMMAND + message_template_key
            4                                                           // wait_seconds_between_repeats
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        // COMMAND + message_template_key
        let mut range: Range<usize> = RangeIterator::new(0);
        serialize_binary_persistable_and_streams_link(Self::INSTANCE.clone(), &self.message_template_key, buffer, &mut range)?;
        // wait_seconds_between_repeats
        range.increment(USIZE_LEN);
        BinaryPersist::to_bytes(&self.wait_seconds_between_repeats, &mut buffer[range.clone()])
            .expect("Serializing wait_seconds_between_repeats failed");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        // COMMAND + message_template_key
        let mut range: Range<usize> = RangeIterator::new(0);
        let mut ret_val = deserialize_enumerated_persistable_arg_with_one_string::<StartSendingMessages, Command>(buffer, &mut range)?;
        // wait_seconds_between_repeats
        range.increment(USIZE_LEN);
        ret_val.wait_seconds_between_repeats = u32::try_from_bytes(&buffer[range]).unwrap();
        Ok(ret_val)
    }
}

