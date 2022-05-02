use std::{
    fmt,
    mem::size_of,
    ops::{
        Range,
    },
};

use anyhow::{
    Result,
    bail,
};

use crate::binary_persist::{
    RangeIterator,
    BinaryPersist,
    USIZE_LEN,
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Command(u8);

impl Command {
    pub const NO_COMMAND: Command = Command(0);
    pub const START_SENDING_MESSAGES: Command = Command(1);
    pub const SUBSCRIBE_TO_ANNOUNCEMENT_LINK: Command = Command(2);
    pub const REGISTER_KEYLOAD_MESSAGE: Command = Command(3);
    pub const PRINTLN_SUBSCRIBER_STATUS: Command = Command(4);
    pub const CLEAR_CLIENT_STATE: Command = Command(5);

    pub const COMMAND_LENGTH_BYTES: usize = size_of::<u8>();

    pub fn as_str(&self) -> &str {
        print_command(self)
    }

    pub fn from_bytes(buffer: &[u8]) -> Result<Command> {
        if buffer.len() != Self::COMMAND_LENGTH_BYTES {
            bail!("Input binary buffer has divergent length of {} bytes. Expected size is {}", buffer.len(), Self::COMMAND_LENGTH_BYTES);
        }
        Ok(Self(u8::try_from_bytes(&buffer[0..Self::COMMAND_LENGTH_BYTES]).unwrap()))
    }

    pub fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        if buffer.len() != Self::COMMAND_LENGTH_BYTES {
            bail!("Input binary buffer has divergent length of {} bytes. Expected size is {}", buffer.len(), Self::COMMAND_LENGTH_BYTES);
        }
        BinaryPersist::to_bytes(&self.0, &mut buffer[0..Self::COMMAND_LENGTH_BYTES])
    }
}

fn print_command(key: &Command) -> &'static str {
    return match key {
        &Command::NO_COMMAND => "NO_COMMAND",                                           // 0
        &Command::START_SENDING_MESSAGES => "START_SENDING_MESSAGES",                   // 1
        &Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK => "SUBSCRIBE_TO_ANNOUNCEMENT_LINK",   // 2
        &Command::REGISTER_KEYLOAD_MESSAGE => "REGISTER_KEYLOAD_Message",               // 3
        &Command::PRINTLN_SUBSCRIBER_STATUS => "PRINTLN_SUBSCRIBER_STATUS",             // 4
        &Command::CLEAR_CLIENT_STATE => "CLEAR_CLIENT_STATE",                           // 5
        _ => "Unknown Command",
    };
}

fn has_command_args(key: &Command) -> bool {
    return match key {
        &Command::START_SENDING_MESSAGES => true,
        &Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK => true,
        &Command::REGISTER_KEYLOAD_MESSAGE => true,
        _ => false,
    };
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            print_command(self)
        )
    }
}

trait CommandArgs {
    const COMMAND: &'static Command;
    fn set_str_arg(&mut self, str_arg: String);
}

fn command_to_bytes(command: Command, link: &String, buffer: &mut [u8])  -> Result<usize> {
    // COMMAND type
    let mut range: Range<usize> = RangeIterator::new(Command::COMMAND_LENGTH_BYTES);
    command.to_bytes(&mut buffer[range.clone()]).expect("Serializing 'COMMAND type' failed");
    let link_bytes = link.as_bytes();
    // Length of persisted link utf8 string binary
    range.increment(USIZE_LEN);
    BinaryPersist::to_bytes(&(link_bytes.len() as u32), &mut buffer[range.clone()]).expect("Serializing 'length of persisted link' failed");
    // announcement_link string utf8 bytes
    range.increment(link_bytes.len());
    buffer[range.clone()].copy_from_slice(link_bytes);
    Ok(range.end)
}

fn command_try_from_bytes<CommandT>(buffer: &[u8] ) -> Result<(CommandT, usize)>
    where
        CommandT: Sized + CommandArgs + Default
{
    let mut range: Range<usize> = RangeIterator::new(Command::COMMAND_LENGTH_BYTES);
    // COMMAND type
    let command = Command::from_bytes(&buffer[range.clone()])?;
    if command != *CommandT::COMMAND {
        bail!("Wrong COMMAND type for deserializing {} instance. Wrong type is {}.", CommandT::COMMAND, command)
    }
    // Length of persisted link utf8 string binary
    range.increment(USIZE_LEN);
    let link_len= u32::try_from_bytes(&buffer[range.clone()]).unwrap();
    // link utf8 string
    range.increment(link_len as usize);
    let link = String::from_utf8(buffer[range.clone()].to_vec())?;
    let mut ret_val = CommandT::default();
    ret_val.set_str_arg(link);
    Ok((ret_val, range.end))
}

fn calc_string_binary_length( str_arg: &String) -> usize {
    str_arg.as_bytes().len() + USIZE_LEN
}

#[derive(Default)]
pub struct SubscribeToAnnouncement {
    pub announcement_link: String,
}

impl CommandArgs for SubscribeToAnnouncement {
    const COMMAND: &'static Command = &Command::SUBSCRIBE_TO_ANNOUNCEMENT_LINK;

    fn set_str_arg(&mut self, str_arg: String) {
        self.announcement_link = str_arg;
    }
}

impl BinaryPersist for SubscribeToAnnouncement {
    fn needed_size(&self) -> usize {
        Command::COMMAND_LENGTH_BYTES + calc_string_binary_length(&self.announcement_link)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        command_to_bytes(Self::COMMAND.clone(), &self.announcement_link, buffer)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let (ret_val, _pos) = command_try_from_bytes::<SubscribeToAnnouncement>(buffer)?;
        Ok(ret_val)
    }
}


#[derive(Default)]
pub struct RegisterKeyloadMessage {
    pub keyload_msg_link: String,
}

impl CommandArgs for RegisterKeyloadMessage {
    const COMMAND: &'static Command = &Command::REGISTER_KEYLOAD_MESSAGE;

    fn set_str_arg(&mut self, str_arg: String) {
        self.keyload_msg_link = str_arg;
    }
}

impl BinaryPersist for RegisterKeyloadMessage {
    fn needed_size(&self) -> usize {
        Command::COMMAND_LENGTH_BYTES + calc_string_binary_length(&self.keyload_msg_link)
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        command_to_bytes(Self::COMMAND.clone(), &self.keyload_msg_link, buffer)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        let (ret_val, _pos) = command_try_from_bytes::<RegisterKeyloadMessage>(buffer)?;
        Ok(ret_val)
    }
}


#[derive(Default)]
pub struct StartSendingMessages {
    pub wait_seconds_between_repeats: u32,
    pub message_template_key: String,
}

impl CommandArgs for StartSendingMessages {
    const COMMAND: &'static Command = &Command::START_SENDING_MESSAGES;

    fn set_str_arg(&mut self, str_arg: String) {
        self.message_template_key = str_arg;
    }
}

impl BinaryPersist for StartSendingMessages {
    fn needed_size(&self) -> usize {
        Command::COMMAND_LENGTH_BYTES +
            calc_string_binary_length(&self.message_template_key) +     // COMMAND + message_template_key
            4                                                           // wait_seconds_between_repeats
    }

    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize> {
        // COMMAND + message_template_key
        let pos = command_to_bytes(Self::COMMAND.clone(), &self.message_template_key, buffer)?;
        let mut range: Range<usize> = RangeIterator::new(pos);
        // wait_seconds_between_repeats
        range.increment(USIZE_LEN);
        BinaryPersist::to_bytes(&self.wait_seconds_between_repeats, &mut buffer[range.clone()])
            .expect("Serializing wait_seconds_between_repeats failed");
        Ok(range.end)
    }

    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized {
        // COMMAND + message_template_key
        let ( mut ret_val, pos) = command_try_from_bytes::<StartSendingMessages>(buffer)?;
        let mut range: Range<usize> = RangeIterator::new(pos);
        // wait_seconds_between_repeats
        range.increment(USIZE_LEN);
        ret_val.wait_seconds_between_repeats = u32::try_from_bytes(&buffer[range]).unwrap();
        Ok(ret_val)
    }
}

