use crate::{
    STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
};

use std::{
    fmt,
};

#[derive(Clone)]
pub struct LoraWanRestClientOptions<'a> {
    pub iota_bridge_url: &'a str,
}

impl Default for LoraWanRestClientOptions<'_> {
    fn default() -> Self {
        Self {
            iota_bridge_url: STREAMS_TOOLS_CONST_IOTA_BRIDGE_URL,
        }
    }
}

impl fmt::Display for LoraWanRestClientOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LoraWanRestClientOptions:\n   iota_bridge_url: {}\n", self.iota_bridge_url)
    }
}