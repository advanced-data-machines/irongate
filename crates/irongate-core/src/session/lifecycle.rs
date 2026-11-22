//! Explicit clock/lease synchronization and immediate denial before cleanup.

use super::{EndReason, Error, Handle, Phase, Table};
use crate::lease;

impl Table<'_> {
    /// Observe the already-advanced lease clock and retain due cleanup in each
    /// slot. No external event buffer is needed or allowed to overflow silently.
    /// The caller still consumes lease effects and owns address reuse ordering.
    pub fn advance(&mut self, leases: &lease::Table<'_>) -> Result<(), Error> {
        self.check_table(leases)?;
        if leases.now() < self.now {
            return Err(Error::ClockRegressed);
        }
        self.now = leases.now();
        for slot in self.slots.iter_mut() {
            let Some(handle) = slot.lease else {
                continue;
            };
            match leases.view(handle) {
                Some(view) if view.phase == lease::Phase::Bound && view.expires_at > self.now => {
                    slot.lease_until = view.expires_at;
                    if slot
                        .attempt
                        .is_some_and(|attempt| attempt.expires_at <= self.now)
                    {
                        slot.terminate(EndReason::ConsentExpired);
                    } else if matches!(slot.phase, Phase::Applying | Phase::Authorized)
                        && slot.authorized_until.is_some_and(|until| until <= self.now)
                    {
                        slot.terminate(EndReason::AuthorizationExpired);
                    }
                }
                _ => slot.terminate(if self.now >= slot.lease_until {
                    EndReason::LeaseExpired
                } else {
                    EndReason::LeaseLost
                }),
            }
        }
        Ok(())
    }

    /// Deny immediately and retain any required removal, even when a lease has
    /// already vanished. Repeated logout preserves the first termination reason.
    pub fn logout(&mut self, handle: Handle) -> Result<(), Error> {
        let index = self.index(handle)?;
        self.slots[index].terminate(EndReason::Logout);
        Ok(())
    }

    /// Consume a RELEASE/DECLINE/expiry notification for the exact lease generation.
    /// Returns the number of matching retained sessions; stale generations do not
    /// revoke replacement sessions. Clock-based expiry is also caught by advance.
    pub fn lease_lost(&mut self, lease: lease::Handle) -> usize {
        let mut count = 0;
        for slot in self
            .slots
            .iter_mut()
            .filter(|slot| slot.lease == Some(lease))
        {
            slot.terminate(EndReason::LeaseLost);
            count += 1;
        }
        count
    }
}
