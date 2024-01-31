use std::{
    collections::VecDeque,
    time::SystemTime
};

use dashmap::{
    DashMap,
};

// Time to wait before a FifoQueueElement can be fetched from the fifo queue (using fifo_queue_pop_front()).
// The time is set to 10 secs because we need to wait for the block being referenced by a milestone
// before it is processed by the streams-collector.
pub static FIFO_MIN_WAIT_TIME_SECS: f32 = 25.0;

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
            if !element.needs_to_wait_for_tangle_milestone {
                ret_val = queue.pop_front();
            } else {
                match element.received.elapsed() {
                    Ok(duration) => {
                        if duration.as_secs_f32() > FIFO_MIN_WAIT_TIME_SECS {
                            ret_val = queue.pop_front();
                        } else {
                            let time_to_wait = duration.as_secs_f32() - FIFO_MIN_WAIT_TIME_SECS;
                            log::info!("[fn fifo_queue_pop_front()] - Minimum wait time has not been reached. time_to_wait: {}", time_to_wait)
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    ret_val
}