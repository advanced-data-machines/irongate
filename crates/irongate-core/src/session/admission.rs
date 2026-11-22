//! Trusted local consent decisions never directly authorize packet forwarding.

use super::{Attempt, EffectKind, Error, Handle, Phase, Table};
use crate::lease;

impl Table<'_> {
    /// Begin/replace a local consent attempt on a live bound lease. Internal
    /// correlation is predictable; a public portal needs separate secure handoff.
    pub fn begin_consent(
        &mut self,
        leases: &lease::Table<'_>,
        handle: Handle,
    ) -> Result<Attempt, Error> {
        self.check_clock(leases)?;
        let index = self.index(handle)?;
        let slot = &self.slots[index];
        self.bound_lease(leases, slot.lease.ok_or(Error::InvalidLease)?)?;
        if !matches!(
            slot.phase,
            Phase::Leased | Phase::AwaitingConsent | Phase::Revoked | Phase::Expired
        ) {
            return Err(Error::InvalidState);
        }
        let expires_at = self
            .now
            .checked_add(u64::from(self.config.consent_seconds))
            .ok_or(Error::ClockOverflow)?;
        let sequence = self.next_sequence()?;
        let attempt = Attempt {
            session: handle,
            sequence,
            expires_at,
        };
        let slot = &mut self.slots[index];
        slot.phase = Phase::AwaitingConsent;
        slot.attempt = Some(attempt);
        slot.authorized_until = None;
        slot.end_reason = None;
        slot.removal_failures = 0;
        Ok(attempt)
    }

    /// Accept a trusted local consent result, retaining an Apply effect. Access
    /// stays denied until its exact dispatched token is acknowledged successfully.
    pub fn consent(
        &mut self,
        leases: &lease::Table<'_>,
        attempt: Attempt,
        seconds: u32,
    ) -> Result<(), Error> {
        self.check_clock(leases)?;
        let index = self
            .index(attempt.session)
            .map_err(|_| Error::StaleAttempt)?;
        let slot = &self.slots[index];
        if slot.phase != Phase::AwaitingConsent
            || slot.attempt != Some(attempt)
            || self.now >= attempt.expires_at
        {
            return Err(Error::StaleAttempt);
        }
        let lease = self.bound_lease(leases, slot.lease.ok_or(Error::InvalidLease)?)?;
        if seconds == 0 || seconds > self.config.max_authorized_seconds {
            return Err(Error::InvalidDuration);
        }
        let until = self
            .now
            .checked_add(u64::from(seconds))
            .ok_or(Error::ClockOverflow)?
            .min(lease.expires_at);
        let slot = &mut self.slots[index];
        slot.attempt = None;
        slot.lease_until = lease.expires_at;
        slot.authorized_until = Some(until);
        slot.phase = Phase::Applying;
        slot.pending = Some(EffectKind::Apply { until });
        Ok(())
    }
}
