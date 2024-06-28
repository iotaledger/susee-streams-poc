use lets::{
    address::{
        Address,
        AppAddr,
        MsgId
    },
    message::TransportMessage
};

use crate::binary_persist::{
    BinaryPersist,
    LinkedMessage
};

const APP_ADDR: [u8; 40] = [170; 40];
const MSGID: [u8; 12] = [255; 12];
const BODY: [u8; 8] = [1,2,3,4,5,6,7,8];

pub (crate) fn get_link() -> Address {
    let appaddr = AppAddr::try_from_bytes(APP_ADDR.as_slice()).expect("deserialize appaddr failed");
    let msgid = MsgId::try_from_bytes(MSGID.as_slice()).expect("deserialize msgid failed");
    Address::new(appaddr, msgid)
}

pub (crate) fn get_transport_message() -> TransportMessage {
    TransportMessage::new(BODY.to_vec())
}

pub (crate) fn get_linked_message() -> LinkedMessage {
    LinkedMessage{
        link: get_link(),
        body: get_transport_message(),
    }
}