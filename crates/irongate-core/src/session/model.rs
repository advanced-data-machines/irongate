//! Public admission state, bounded failures and policy requests.

use super::{Attempt, EffectToken, Handle};
use crate::lease::{self, NetworkId};

/// Pilot bound on scans and caller-owned session slots per network.
pub const MAX_SESSIONS: usize = 256;

/// Construction policy for one network and one authoritative lease arena.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Trusted ingress network.
    pub network: NetworkId,
    /// Nonzero owner epoch, unique across construction/reload.
    pub epoch: u64,
    /// Maximum time to complete a consent or external authentication attempt.
    pub consent_seconds: u32,
    /// Maximum requested access lifetime; also capped by the current lease.
    pub max_authorized_seconds: u32,
}

/// Explicit admission failures; no strings or allocation required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Invalid epoch, duration or slot count.
    InvalidConfig,
    /// Only empty/retired slots can be borrowed by a new owner.
    InvalidStorage,
    /// Supplied lease arena differs from the configured network/epoch.
    WrongLeaseTable,
    /// Lease clock must be advanced before the session clock; commands require equality.
    ClockMismatch,
    /// Time moved backward.
    ClockRegressed,
    /// Checked deadline arithmetic overflowed.
    ClockOverflow,
    /// No free usable session slot; existing sessions were not evicted.
    Exhausted,
    /// An older session at this address still owns admission/cleanup work.
    AddressBusy,
    /// Lease is absent, expired or not bound.
    InvalidLease,
    /// Session epoch/network/generation no longer resolves.
    StaleHandle,
    /// Admission attempt expired, was replaced or belongs to another session.
    StaleAttempt,
    /// Policy completion does not match the currently dispatched operation.
    StaleEffect,
    /// Admission or freeing is incompatible with the current phase/effects.
    InvalidState,
    /// Requested lifetime is zero or exceeds configured policy.
    InvalidDuration,
    /// Correlation sequences never wrap; reconstruct the owner with a new epoch.
    SequenceExhausted,
}

/// Source of a validated admission decision; credentials never enter the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// Local acceptance of network terms.
    Consent,
    /// A validated result from the configured RADIUS worker.
    Radius,
}

/// Last observed session state; `allows` separately checks current lease/time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Address attached, upstream denied.
    Leased,
    /// A consent attempt is outstanding, upstream denied.
    AwaitingConsent,
    /// An external authentication attempt is outstanding, upstream denied.
    AwaitingAuthentication,
    /// Positive decision awaits policy application acknowledgement, upstream denied.
    Applying,
    /// Policy application confirmed and access lifetime remains valid.
    Authorized,
    /// Upstream denied; retain storage until policy removal is confirmed.
    Revoking,
    /// Closed with policy removal confirmed or no policy ever dispatched.
    Revoked,
    /// Consent, lease or authorization expired and cleanup has finished.
    Expired,
}

/// Retained reason for closing/denying this admission lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// Explicit local logout.
    Logout,
    /// Lease released, declined or otherwise ceased to be a bound lease.
    LeaseLost,
    /// Last observed lease deadline elapsed.
    LeaseExpired,
    /// Admitted lifetime elapsed.
    AuthorizationExpired,
    /// Consent attempt deadline elapsed.
    ConsentExpired,
    /// Policy application failed; removal must clean up possible partial work.
    ApplyFailed,
    /// Backend explicitly rejected the current authentication attempt.
    AuthenticationRejected,
    /// Authentication failed or the supported policy could not be applied.
    AuthenticationFailed,
    /// The current authentication attempt exceeded its deadline.
    AuthenticationExpired,
}

impl EndReason {
    pub(super) fn terminal(self) -> Phase {
        match self {
            Self::LeaseExpired
            | Self::AuthorizationExpired
            | Self::ConsentExpired
            | Self::AuthenticationExpired => Phase::Expired,
            Self::Logout
            | Self::LeaseLost
            | Self::ApplyFailed
            | Self::AuthenticationRejected
            | Self::AuthenticationFailed => Phase::Revoked,
        }
    }
}

/// Idempotent policy action. One pending/in-flight operation per reserved slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    /// Install access bounded by this monotonic deadline.
    Apply {
        /// Access must cease by this time even if the owner/worker is delayed.
        until: u64,
    },
    /// Remove this session's policy before acknowledging completion.
    Remove {
        /// Reason retained through cleanup and retries.
        reason: EndReason,
    },
}

/// Dispatched operation with exact session and lease identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Effect {
    /// Acknowledge this token after the operation is finished/quiescent.
    pub token: EffectToken,
    /// Lease generation being controlled, not just an IP address.
    pub lease: lease::Handle,
    /// Address for a future adapter; never sufficient identity by itself.
    pub address: [u8; 4],
    /// Requested action.
    pub kind: EffectKind,
}

/// Bounded diagnostic snapshot, not durable accounting or proof of wire enforcement.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// Session identity.
    pub handle: Handle,
    /// Bound lease identity.
    pub lease: lease::Handle,
    /// Client address.
    pub address: [u8; 4],
    /// Current owner phase.
    pub phase: Phase,
    /// Source of the current/last applied positive decision, if any.
    pub provider: Option<Provider>,
    /// Last observed lease deadline.
    pub lease_until: u64,
    /// Access deadline after a positive decision, if present.
    pub authorized_until: Option<u64>,
    /// Current consent attempt, if present.
    pub attempt: Option<Attempt>,
    /// Work retained for future dispatch.
    pub pending: Option<EffectKind>,
    /// Dispatched operation awaiting a final acknowledgement.
    pub in_flight: Option<Effect>,
    /// Retained termination/denial reason.
    pub end_reason: Option<EndReason>,
    /// Failed removal attempts, saturating rather than wrapping.
    pub removal_failures: u32,
}
