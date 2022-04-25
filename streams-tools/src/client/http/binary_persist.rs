use std::{
    ops::Range,
};

use anyhow::Result;

pub trait RangeIterator<Idx> {
    fn new(first_length: Idx) -> Self;
    fn increment(&mut self, next_length: Idx);
}

impl RangeIterator<usize> for Range<usize> {
    fn new(first_length: usize) -> Self {
        Self {
            start: 0usize,
            end: first_length,
        }
    }
    fn increment(&mut self, next_length: usize) {
        self.start = self.end.clone();
        self.end = self.end.clone() + next_length;
    }
}

// Whenever the size of data is persisted into a binary buffer we will use 4 bytes for the length
// information independent from the usize of the system
pub static USIZE_LEN: usize = 4;

pub trait BinaryPersist {
    fn needed_size(&self) -> usize;
    fn to_bytes(&self, buffer: &mut [u8]) -> Result<usize>;

    // static
    fn try_from_bytes(buffer: &[u8]) -> Result<Self> where Self: Sized;
}