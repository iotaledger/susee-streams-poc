use std::{
    collections::VecDeque,
    time::SystemTime
};

use dashmap::{
    DashMap,
};

use crate::streams_transport::streams_transport::STREAMS_TOOLS_CONST_TRANSPORT_PROCESSING_TIME_SECS;

// Time to wait before a FifoQueueElement can be fetched from the fifo queue (using fifo_queue_pop_front())
// in case FifoQueueElement::needs_to_wait_for_tangle_milestone is true.
// -------------------------------------------------------------------------------------------
// -----------------------------     IMPORTANT      ------------------------------------------
// -------------------------------------------------------------------------------------------
//      If DispatchStreams::send_message() calls transport.send_message()
//      and awaits the response, we do not to wait in fifo_queue_pop_front()
//      because new commands/confirmations will always been posted after
//      blocks have been fully processed by the transport services.
//      Otherwise: If DispatchStreams::send_message() calls transport.send_message()
//      in a sub-task and returns early without waiting for the response,
//      we would need to set FIFO_MIN_WAIT_TIME_SECS to
//      STREAMS_TOOLS_CONST_TRANSPORT_PROCESSING_TIME_SECS.
//      This option is currently not implemented so that we set FIFO_MIN_WAIT_TIME_SECS
//      to 0.1 seconds here.
// -------------------------------------------------------------------------------------------
pub static FIFO_MIN_WAIT_TIME_SECS: f32 = 0.1;

// Lifetime of a FifoQueueElement. If the FIFO_ELEMENT_LIFETIME_SECS has expired, the FifoQueueElement
// will not be delivered. Instead it is popped of the queue and dropped.
pub static FIFO_ELEMENT_LIFETIME_SECS: f32 = 600.0;

// This is used to store Commands and Confirmations in the FifoQueue
pub struct FifoQueueElement {
    pub payload: Vec<u8>,
    pub received: SystemTime,
    pub needs_to_wait_for_tangle_milestone: bool,
}

impl FifoQueueElement {
    pub fn from_binary(binary: &[u8], needs_to_wait: bool) -> FifoQueueElement {
        FifoQueueElement {
            payload: binary.to_vec(),
            received: SystemTime::now(),
            needs_to_wait_for_tangle_milestone: needs_to_wait
        }
    }
}

pub type FifoQueue = VecDeque<FifoQueueElement>;

pub type FifoQueueMap = DashMap<String, FifoQueue>;

pub fn get_new_fifo_queue_map() -> FifoQueueMap {
    DashMap::with_capacity(1)
}

pub fn create_new_fifo_queue_if_not_exist(fifos: &FifoQueueMap,dev_eui: &str) {
    if !fifos.contains_key(dev_eui) {
        fifos.insert(dev_eui.to_string(), FifoQueue::new());
    }
}

pub fn fifo_queue_pop_front(queue: &mut FifoQueue) -> Option<FifoQueueElement> {
    let mut ret_val: Option<FifoQueueElement> = None;
    if !queue.is_empty() {
        if let Some(element) = queue.get(0) {
            match element.received.elapsed() {
                Ok(duration) => {
                    if duration.as_secs_f32() < FIFO_ELEMENT_LIFETIME_SECS {
                        if !element.needs_to_wait_for_tangle_milestone {
                            ret_val = queue.pop_front();
                        } else {
                            if duration.as_secs_f32() > FIFO_MIN_WAIT_TIME_SECS {
                                ret_val = queue.pop_front();
                            } else {
                                let time_to_wait = duration.as_secs_f32() - FIFO_MIN_WAIT_TIME_SECS;
                                log::info!("[fn fifo_queue_pop_front()] - Minimum wait time has not been reached. time_to_wait: {}", time_to_wait)
                            }
                        }
                    } else {
                        let lifetime_secs = duration.as_secs_f32();
                        log::info!("[fn fifo_queue_pop_front()] - The maximum lifetime of {} secs has been exceeded by a FifoQueueElement. The element has been dropped. Lifetime has been: {} secs", FIFO_ELEMENT_LIFETIME_SECS, lifetime_secs);
                        let _ = queue.pop_front();
                    }
                }
                _ => {}
            }
        }
    }
    ret_val
}