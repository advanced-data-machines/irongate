//! Lease state and effects; deliberately separate from access authorization.

use super::{ClientKey, NetworkId};

/// Explicit bounded-state failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Pool addresses, durations, epoch or prefix are invalid.
    InvalidConfig,
    /// Client hardware/opaque identifier is invalid.
    InvalidClient,
    /// Storage must have exactly one empty slot per address.
    InvalidStorage,
    /// The caller selected the wrong pool for a client/network.
    WrongNetwork,
    /// Every usable slot is reserved, bound, quarantined or retired.
    Exhausted,
    /// Monotonic time moved backward.
    ClockRegressed,
    /// A future deadline would overflow; no wrapping timers.
    ClockOverflow,
    /// Expiry effects must fit before any clock/state mutation occurs.
    EventCapacity {
        /// Number of event cells required for this advance.
        required: usize,
    },
}

/// Public observable address state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Reserved by a DISCOVER, not leased yet.
    Offered,
    /// Bound by REQUEST/ACK, without portal authorization.
    Bound,
    /// Client reported conflict; unavailable until quarantine ends.
    Declined,
}

/// Generation-safe internal reference, valid only in the matching arena epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub(super) network: NetworkId,
    pub(super) epoch: u64,
    pub(super) index: u16,
    pub(super) generation: u64,
}

impl Handle {
    /// Subscriber network.
    pub fn network(self) -> NetworkId {
        self.network
    }
    /// Generation changes when a freed slot is allocated again; never wraps.
    pub fn generation(self) -> u64 {
        self.generation
    }
}

/// A read-only snapshot, borrowed from current live storage.
#[derive(Debug)]
pub struct View<'a> {
    /// Internal identity; not sent on the DHCP wire.
    pub handle: Handle,
    /// Address owned by this slot.
    pub address: [u8; 4],
    /// Ownership/quarantine state.
    pub phase: Phase,
    /// Client is absent during conflict quarantine.
    pub client: Option<&'a ClientKey>,
    /// Current state expiry in monotonic seconds.
    pub expires_at: u64,
}

/// Address and correlation returned for OFFER or ACK; not a forwarding grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grant {
    /// Internal correlation, including generation.
    pub handle: Handle,
    /// Allocated address.
    pub address: [u8; 4],
    /// Reply lease duration. Duplicate SELECTING ACKs use remaining lifetime.
    pub lease_seconds: u32,
}

/// DHCPREQUEST forms, after wire-field consistency has been checked by an adapter.
#[derive(Clone, Copy, Debug)]
pub enum RequestMode {
    /// Selecting an offered server/address.
    Selecting {
        /// Chosen server identifier.
        server: [u8; 4],
        /// Requested address.
        address: [u8; 4],
    },
    /// Checking a cached binding after client restart.
    InitReboot {
        /// Cached address (option 50).
        address: [u8; 4],
    },
    /// RENEWING/REBINDING share ownership checks; transport chooses reply delivery.
    Renew {
        /// Current client address (ciaddr).
        address: [u8; 4],
    },
}

/// Validated direct-client command; wire decoding and routing are separate.
#[derive(Clone, Copy, Debug)]
pub enum Command<'a> {
    /// Reserve/reuse an offer; requested address is a hint only.
    Discover {
        /// Network-scoped identity.
        client: &'a ClientKey,
        /// Transaction identifier.
        xid: u32,
        /// Client address hint.
        requested: Option<[u8; 4]>,
    },
    /// Bind or renew an address.
    Request {
        /// Network-scoped identity.
        client: &'a ClientKey,
        /// Transaction identifier.
        xid: u32,
        /// Client request phase.
        mode: RequestMode,
    },
    /// Release only the same client's active binding, naming our server.
    Release {
        /// Network-scoped identity.
        client: &'a ClientKey,
        /// Address being released.
        address: [u8; 4],
        /// Server identifier.
        server: [u8; 4],
    },
    /// Quarantine only the same client's offered/bound address.
    Decline {
        /// Network-scoped identity.
        client: &'a ClientKey,
        /// Conflicting address.
        address: [u8; 4],
        /// Server identifier.
        server: [u8; 4],
    },
}

/// A response must remain silent for these conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IgnoreReason {
    /// Request selected a different server.
    OtherServer,
    /// No matching ownership record.
    NoBinding,
    /// Request does not match the current client's binding.
    NotOwner,
}

/// Reasons for a negative reply under this pool's authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NakReason {
    /// Requested address is invalid/on a different subnet.
    WrongAddress,
    /// No current matching offer/transaction.
    NoOffer,
    /// Known client requested a different/occupied address.
    BindingMismatch,
}

/// One synchronous lease effect. RELEASE/DECLINE never generate a DHCP reply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Reserve or reiterate an offer.
    Offer(Grant),
    /// Create/extend/reiterate a binding.
    Ack(Grant),
    /// Encode a negative response.
    Nak(NakReason),
    /// Send no response.
    Ignore(IgnoreReason),
    /// Binding removed; owner must consume the effect.
    Released(Handle),
    /// Address quarantined; owner must consume the effect.
    Declined(Handle),
}

/// Expiry events returned before a slot may be reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// Unaccepted offer expired.
    OfferExpired,
    /// A pending rediscovery offer expired; original binding remains.
    PendingOfferExpired,
    /// Active lease expired; revoke dependent session state.
    BindingExpired,
    /// Conflict hold ended; address can be offered again.
    QuarantineEnded,
}

/// A due event must be consumed even when no packet reply is sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// Address generation that produced the event.
    pub handle: Handle,
    /// Monotonic transition reason.
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Free,
    Offered,
    Bound,
    Declined,
    Retired,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Pending {
    pub xid: u32,
    pub expires: u64,
}

/// One caller-owned address slot. Keep large slot arrays in startup-owned storage.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub(super) generation: u64,
    pub(super) state: State,
    pub(super) client: Option<ClientKey>,
    pub(super) expires: u64,
    pub(super) pending: Option<Pending>,
    pub(super) last_xid: u32,
}

impl Slot {
    /// Initial empty storage. Generation exhaustion permanently retires a slot.
    pub const EMPTY: Self = Self {
        generation: 0,
        state: State::Free,
        client: None,
        expires: 0,
        pending: None,
        last_xid: 0,
    };
    pub(super) fn clear(&mut self) {
        *self = Self {
            generation: self.generation,
            ..Self::EMPTY
        };
    }
}
