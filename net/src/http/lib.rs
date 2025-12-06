use std::sync::atomic::{AtomicU64, Ordering};

pub struct OrderIdGenerator {
    counter: AtomicU64,
}

impl OrderIdGenerator {
    pub fn new(start_id: u64) -> Self {
        Self {
            counter: AtomicU64::new(start_id),
        }
    }

    pub fn next(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for OrderIdGenerator {
    fn default() -> Self {
        Self::new(1)
    }
}
