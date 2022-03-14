

#[derive(Clone, PartialEq, Eq, Hash)]
enum ArgKeyInner {
    ContentToSend,
    SubscribeAnnouncementLink,
    RegisterKeyloadMsg,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArgKeys(ArgKeyInner);

impl ArgKeys {
    pub const CONTENT_TO_SEND: ArgKeys = ArgKeys(ArgKeyInner::ContentToSend);
    pub const SUBSCRIBE_ANNOUNCEMENT_LINK: ArgKeys = ArgKeys(ArgKeyInner::SubscribeAnnouncementLink);
    pub const REGISTER_KEYLOAD_MSG: ArgKeys = ArgKeys(ArgKeyInner::RegisterKeyloadMsg);
}

fn lookup_value(key: ArgKeys) -> &'static str {
    return match key {
        ArgKeys::CONTENT_TO_SEND => "",
        ArgKeys::SUBSCRIBE_ANNOUNCEMENT_LINK => "",
        ArgKeys::REGISTER_KEYLOAD_MSG => "",
    };
}

fn lookup_value_is_set(key: ArgKeys) -> bool {
    return match key {
        ArgKeys::CONTENT_TO_SEND => true,
        _ => false,
    };
}

pub struct ProcessingArgs {}

impl ProcessingArgs {
    pub fn contains_key(key: ArgKeys) -> bool{
        lookup_value_is_set(key)
    }

    pub fn get(key: ArgKeys) -> &'static str {
        lookup_value(key)
    }
}