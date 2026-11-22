//! Serialize external work per session; failure never drops cleanup obligations.

use super::{Effect, EffectKind, EffectToken, EndReason, Error, Phase, Table};
use crate::lease;

impl Table<'_> {
    /// Dispatch at most one retained operation, fairly scanning fixed slots.
    /// No second operation for a session is dispatched before final completion.
    /// A worker must treat operations as idempotent and acknowledge only after
    /// they have finished/quiesced; a timeout alone does not prove that.
    pub fn take_effect(&mut self, leases: &lease::Table<'_>) -> Result<Option<Effect>, Error> {
        self.check_clock(leases)?;
        for offset in 0..self.slots.len() {
            let index = (self.dispatch_cursor + offset) % self.slots.len();
            let slot = &self.slots[index];
            let Some(kind) = slot.pending else {
                continue;
            };
            if slot.in_flight.is_some() {
                continue;
            }
            let lease = slot.lease.expect("pending effects retain their session");
            if let EffectKind::Apply { until } = kind {
                let reason = if self.bound_lease(leases, lease).is_err() {
                    Some(if self.now >= slot.lease_until {
                        EndReason::LeaseExpired
                    } else {
                        EndReason::LeaseLost
                    })
                } else if self.now >= until {
                    Some(EndReason::AuthorizationExpired)
                } else {
                    None
                };
                if let Some(reason) = reason {
                    self.slots[index].terminate(reason);
                    continue;
                }
            }
            let sequence = self.next_sequence()?;
            let effect = Effect {
                token: EffectToken {
                    session: self.handle(index),
                    sequence,
                },
                lease,
                address: self.slots[index].address,
                kind,
            };
            let slot = &mut self.slots[index];
            slot.pending = None;
            slot.in_flight = Some(effect);
            self.dispatch_cursor = (index + 1) % self.slots.len();
            return Ok(Some(effect));
        }
        Ok(None)
    }

    /// Record a final, trusted policy-worker result. Failure may include partial
    /// external changes: failed Apply always requires Remove; failed Remove is
    /// retained for retry with a new token. Stale/duplicate tokens change nothing.
    pub fn complete(
        &mut self,
        leases: &lease::Table<'_>,
        token: EffectToken,
        success: bool,
    ) -> Result<(), Error> {
        self.check_clock(leases)?;
        let index = self.effect_index(token)?;
        let slot = &self.slots[index];
        let effect = slot.in_flight.expect("effect_index validates a flight");
        let live = self.bound_lease(leases, effect.lease).is_ok();
        let slot = &mut self.slots[index];
        slot.in_flight = None;
        match effect.kind {
            EffectKind::Apply { until } => {
                if success && live && self.now < until && slot.phase == Phase::Applying {
                    slot.phase = Phase::Authorized;
                } else {
                    let reason = slot.end_reason.unwrap_or(if !live {
                        EndReason::LeaseLost
                    } else if self.now >= until {
                        EndReason::AuthorizationExpired
                    } else {
                        EndReason::ApplyFailed
                    });
                    slot.end_reason = Some(reason);
                    slot.phase = Phase::Revoking;
                    slot.pending = Some(EffectKind::Remove { reason });
                }
            }
            EffectKind::Remove { reason } => {
                if success {
                    slot.phase = reason.terminal();
                } else {
                    slot.removal_failures = slot.removal_failures.saturating_add(1);
                    slot.pending = Some(effect.kind);
                }
            }
        }
        Ok(())
    }
}
