//! Bounded, allocator-independent network access foundations.
//!
//! The DHCP lease engine borrows caller-owned storage and an explicit clock.
//! Leases do not authorize access. Portal sessions and enforcement are future work.

#![no_std]
#![forbid(unsafe_code)]

pub mod lease;
