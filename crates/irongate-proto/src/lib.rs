//! Borrowed, bounded direct-client DHCP protocol experiment.
//!
//! This is an offline pilot, not a DHCP server. No leases, sockets or policy.
//! Input is Ethernet II without FCS; output borrows caller-owned storage.

#![no_std]
#![forbid(unsafe_code)]

mod checksum;
pub mod dhcp;
mod error;
pub mod ethernet;
pub mod ipv4_udp;
mod request;

pub use error::{Error, Layer};
pub use request::{RequestFrame, parse_request};

/// Maximum accepted input frame bytes, including any Ethernet padding.
pub const MAX_FRAME_BYTES: usize = 2048;
