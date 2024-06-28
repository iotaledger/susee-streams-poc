use anyhow::{
    Result,
};
use std::{
    rc::Rc,
    cell::Cell,
};

// The use_compressed_msg state indicates if a Subscriber is known by the iota-bridge so that
// compressed messages can be used. Subscribers will send uncompressed messages
// until the iota-bridge indicates that it has stored all needed data to use
// compressed massages further on (see streams-tools/src/iota_bridge/server_dispatch_streams.rs
// for more details).
// The SubscriberManager persists the use_compressed_msg state in its Streams Client State
// serialization file so that the state will not get lost.
//
// As the iota-bridge indicates the use_compressed_msg state using the specific transport
// protocol implemented by the ClientTTrait instance (example: http) a use_compressed_msg
// state change is signaled to the SubscriberManager via the CompressedStateListen
// trait by the ClientTTrait implementation that implements the streams transport.
pub trait CompressedStateListen {
    fn set_use_compressed_msg(&self, use_compressed_msg: bool);
    fn get_use_compressed_msg(&self) -> bool;
}

// Used to register CompressedStateListen listeners at ClientTTrait implementations.
pub trait CompressedStateSend {
    // Returns the handle to be used with remove_listener() to unsubscribe the listener
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize>;
    // Use this to initialize the ClientTTrait implementation before any iota-bridge communication
    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool);
    // Unsubscribe the listener. The handle has been obtained as result from the subscribe_listener() call
    fn remove_listener(&mut self, handle: usize);
}

pub struct CompressedStateManager {
    use_compressed_msg: Cell<bool>,
    listeners: Vec<Rc<dyn CompressedStateListen>>,
    is_cloned: bool,
}

// Cloning a CompressedStateManager including a filled listeners vec can lead to chaotic notifications
// because it will not be clear which of the cloned CompressedStateManagers has send a notification.
// The intended use case for cloning a CompressedStateManager is to have an instance that is used as
// template for ONE further cloned instance that acts as the real publisher notifying the listeners.
// This is needed because ClientTTrait implementations are non public fields in Streams subscribers
// and can only be accessed read only.
impl Clone for CompressedStateManager {
    fn clone(&self) -> Self {
        let mut ret_val = CompressedStateManager::new();
        ret_val.clone_from(self);
        ret_val
    }

    fn clone_from(&mut self, source: &Self) {
        if source.is_cloned {
            panic!("It is not allowed to clone a CompressedStateManager instance that has been cloned itself.")
        }

        let mut listeners_have_already_been_cloned = false;
        if let Some(first_listener) = source.listeners.first() {
            // Listeners reference count (strong_count) will be at least 2.
            // +1 for the SubscriberManager and +1 for this CompressedStateManager itself.
            // In case a CompressedStateManager has already been cloned to be used in a
            // Streams Subscriber the strong_count will be > 2
            listeners_have_already_been_cloned = Rc::strong_count(first_listener) > 2;
        }

        if !listeners_have_already_been_cloned {
            self.use_compressed_msg.set(source.use_compressed_msg.get());
            self.listeners = source.listeners.clone();
            self.is_cloned = true;
        } else {
            panic!("This CompressedStateManager instance has already been cloned. You can clone CompressedStateManager only once")
        }
    }
}

impl CompressedStateManager {
    pub fn new() -> Self {
        Self {
            use_compressed_msg: Cell::new(false),
            listeners: Vec::new(),
            is_cloned: false,
        }
    }
}

impl CompressedStateSend for CompressedStateManager {
    fn subscribe_listener(&mut self, listener: Rc<dyn CompressedStateListen>) -> Result<usize> {
        listener.set_use_compressed_msg(self.use_compressed_msg.get());
        self.listeners.push(listener);
        Ok(self.listeners.len() - 1)
    }

    fn set_initial_use_compressed_msg_state(&self, use_compressed_msg: bool) {
        self.set_use_compressed_msg(use_compressed_msg);
    }

    fn remove_listener(&mut self, handle: usize) {
        self.listeners.remove(handle);
    }
}

impl CompressedStateListen for CompressedStateManager {
    fn set_use_compressed_msg(&self, use_compressed_msg: bool) {
        self.use_compressed_msg.set(use_compressed_msg);
        for listener in &self.listeners {
            listener.set_use_compressed_msg(use_compressed_msg);
        }
    }

    fn get_use_compressed_msg(&self) -> bool {
        self.use_compressed_msg.get()
    }
}