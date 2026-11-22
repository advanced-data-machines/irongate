//! Offline direct-client leases. Advance time and consume expiry effects before
//! handling commands; all durations use caller-supplied monotonic seconds.

mod client;
mod config;
mod maintenance;
mod model;
mod requests;
mod table;

pub use client::{ClientIdentity, ClientKey, NetworkId};
pub use config::{MAX_LEASES, PoolConfig};
pub use model::{
    Command, Error, Event, EventKind, Grant, Handle, IgnoreReason, NakReason, Outcome, Phase,
    RequestMode, Slot, View,
};
pub use table::Table;

#[cfg(test)]
mod tests;
