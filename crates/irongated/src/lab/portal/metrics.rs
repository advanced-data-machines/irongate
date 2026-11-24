//! Bounded worker observations; no per-connection event collection.
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
#[derive(Default)]
pub(crate) struct Metrics {
    pub accepted: AtomicU64,
    pub completed: AtomicU64,
    pub failed: AtomicU64,
    pub timed_out: AtomicU64,
    pub busy: AtomicU64,
}
#[derive(Clone, Copy)]
pub(crate) struct Health {
    pub secure: bool,
    pub accepted: u64,
    pub completed: u64,
    pub failed: u64,
    pub timed_out: u64,
    pub busy: u64,
}
impl Metrics {
    pub fn snapshot(&self, secure: bool) -> Health {
        Health {
            secure,
            accepted: self.accepted.load(Relaxed),
            completed: self.completed.load(Relaxed),
            failed: self.failed.load(Relaxed),
            timed_out: self.timed_out.load(Relaxed),
            busy: self.busy.load(Relaxed),
        }
    }
}
