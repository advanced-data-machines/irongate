//! Internal correlation identities, never browser authentication credentials.

use crate::lease::NetworkId;

/// Internal session identity, valid in one owner epoch and slot generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub(super) network: NetworkId,
    pub(super) epoch: u64,
    pub(super) index: u16,
    pub(super) generation: u64,
}

impl Handle {
    /// Network identity, independent of client MAC or portal metadata.
    pub fn network(self) -> NetworkId {
        self.network
    }
    /// Slot generation, incremented on reuse and never wrapped.
    pub fn generation(self) -> u64 {
        self.generation
    }
}

/// One bounded consent attempt. Correlates trusted worker work, not public HTTP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub(super) session: Handle,
    pub(super) sequence: u64,
    pub(super) expires_at: u64,
}

impl Attempt {
    /// Session that initiated this attempt.
    pub fn session(self) -> Handle {
        self.session
    }
    /// Monotonic consent deadline.
    pub fn expires_at(self) -> u64 {
        self.expires_at
    }
}

/// One dispatched policy operation; retries always receive a new sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectToken {
    pub(super) session: Handle,
    pub(super) sequence: u64,
}

impl EffectToken {
    /// Session whose policy is being changed.
    pub fn session(self) -> Handle {
        self.session
    }
}
