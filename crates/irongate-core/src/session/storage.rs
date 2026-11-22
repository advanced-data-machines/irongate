//! Each occupied session reserves its own policy-effect cell.

use super::{Attempt, Effect, EffectKind, EndReason, Phase};
use crate::lease;

/// One caller-owned session slot; fields can only be mutated by its table.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub(super) generation: u64,
    pub(super) retired: bool,
    pub(super) lease: Option<lease::Handle>,
    pub(super) address: [u8; 4],
    pub(super) phase: Phase,
    pub(super) lease_until: u64,
    pub(super) authorized_until: Option<u64>,
    pub(super) attempt: Option<Attempt>,
    pub(super) pending: Option<EffectKind>,
    pub(super) in_flight: Option<Effect>,
    pub(super) end_reason: Option<EndReason>,
    pub(super) removal_failures: u32,
}

impl Slot {
    /// Initially unused storage. Generation/retirement survive clearing.
    pub const EMPTY: Self = Self {
        generation: 0,
        retired: false,
        lease: None,
        address: [0; 4],
        phase: Phase::Leased,
        lease_until: 0,
        authorized_until: None,
        attempt: None,
        pending: None,
        in_flight: None,
        end_reason: None,
        removal_failures: 0,
    };

    pub(super) fn clear(&mut self) {
        *self = Self {
            generation: self.generation,
            retired: self.retired,
            ..Self::EMPTY
        };
    }

    pub(super) fn closed(&self) -> bool {
        matches!(self.phase, Phase::Revoked | Phase::Expired)
            && self.pending.is_none()
            && self.in_flight.is_none()
    }

    pub(super) fn terminate(&mut self, reason: EndReason) {
        if self.closed() || self.phase == Phase::Revoking {
            return;
        }
        self.attempt = None;
        self.end_reason = Some(reason);
        if self.phase == Phase::Authorized {
            self.phase = Phase::Revoking;
            self.pending = Some(EffectKind::Remove { reason });
        } else if self.in_flight.is_some() {
            // A dispatched Apply must finish before any Remove can be dispatched.
            self.phase = Phase::Revoking;
        } else {
            // Unsent Apply can be cancelled without any external cleanup.
            self.pending = None;
            self.phase = reason.terminal();
        }
    }
}
