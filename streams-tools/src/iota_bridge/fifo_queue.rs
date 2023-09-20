use std::{
    collections::VecDeque,
    time::SystemTime
};

// Time to wait before a FifoQueueElement can be fetched from the fifo queue (using fifo_queue_pop_front()).
// The time is set to 10 secs because we need to wait for the block being referenced by a milestone
// before it is processed by the streams-collector.
pub static FIFO_MIN_WAIT_TIME_SECS: f32 = 10.0;

// This is used to store Commands and Confirmations in the FifoQueue
pub struct FifoQueueElement {
    pub payload: Vec<u8>,
    pub received: SystemTime,
}

impl FifoQueueElement {
    pub fn from_binary(binary: &[u8]) -> FifoQueueElement {
        FifoQueueElement {
            payload: binary.to_vec(),
            received: SystemTime::now(),
        }
    }
}

pub type FifoQueue = VecDeque<FifoQueueElement>;

pub fn fifo_queue_pop_front(queue: &mut FifoQueue) -> Option<FifoQueueElement> {
    let mut ret_val: Option<FifoQueueElement> = None;
    if !queue.is_empty() {
        if let Some(element) = queue.get(0) {
            match element.received.elapsed() {
                Ok(duration) => {
                    if duration.as_secs_f32() > FIFO_MIN_WAIT_TIME_SECS {
                        ret_val = queue.pop_front();
                    }
                }
                _ => {}
            }
        }
    }
    ret_val
}