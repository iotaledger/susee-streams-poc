
static METER_READING_1: &[u8] = include_bytes!("meter_reading_1.json");
static METER_READING_1_COMPACT: &[u8] = include_bytes!("meter_reading_1_compact.json");


#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Message<'a>(&'a str);

impl<'a> Message<'a> {
    pub const UNKNOWN_MESSAGE_KEY: Message<'a> = Message("Unknown Message Key");
    pub const METER_READING_1: Message<'a> = Message("meter_reading_1.json");
    pub const METER_READING_1_COMPACT: Message<'a> = Message("meter_reading_1_compact.json");
}

impl<'a> From<&'a str> for Message<'a> {
    fn from(key: &'a str) -> Self {
        Message::<'a>(key)
    }
}

impl<'a> Into<&'a str> for Message<'a> {
    fn into(self) -> &'a str {
        self.0
    }
}

pub fn get_message_bytes(key: Message) -> &[u8] {
    match key {
        Message::METER_READING_1 => METER_READING_1,
        Message::METER_READING_1_COMPACT => METER_READING_1_COMPACT,
        _ => Message::UNKNOWN_MESSAGE_KEY.0.as_bytes(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_message_bytes() {
        assert_eq!(get_message_bytes(Message::from("This is nonsense")), Message::UNKNOWN_MESSAGE_KEY.0.as_bytes());
        assert_eq!(get_message_bytes(Message::from("meter_reading_1.json")), METER_READING_1);
        assert_eq!(get_message_bytes(Message::from("meter_reading_1_compact.json")), METER_READING_1_COMPACT);
    }
}
