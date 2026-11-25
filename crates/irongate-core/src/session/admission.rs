//! Trusted local consent decisions never directly authorize packet forwarding.

use super::{Attempt, EffectKind, EndReason, Error, Handle, Phase, Provider, Table};
use crate::lease;

impl Table<'_> {
    /// Begin/replace a local consent attempt on a live bound lease. Internal
    /// correlation is predictable; a public portal needs separate secure handoff.
    pub fn begin_consent(
        &mut self,
        leases: &lease::Table<'_>,
        handle: Handle,
    ) -> Result<Attempt, Error> {
        self.begin(leases, handle, Phase::AwaitingConsent)
    }

    /// Begin an external RADIUS attempt without granting access. Replacing an
    /// attempt invalidates its sequence; logout and lease loss cancel it.
    pub fn begin_radius(
        &mut self,
        leases: &lease::Table<'_>,
        handle: Handle,
    ) -> Result<Attempt, Error> {
        self.begin(leases, handle, Phase::AwaitingAuthentication)
    }

    fn begin(
        &mut self,
        leases: &lease::Table<'_>,
        handle: Handle,
        phase: Phase,
    ) -> Result<Attempt, Error> {
        self.check_clock(leases)?;
        let index = self.index(handle)?;
        let slot = &self.slots[index];
        self.bound_lease(leases, slot.lease.ok_or(Error::InvalidLease)?)?;
        if !matches!(slot.phase, Phase::Leased | Phase::Revoked | Phase::Expired)
            && slot.phase != phase
        {
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
        slot.phase = phase;
        slot.provider = None;
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
        self.authorize(leases, attempt, seconds, Provider::Consent)
    }

    /// Accept only the current RADIUS attempt. The adapter must first validate
    /// the backend response and supported policy. Exact Apply ack is still needed.
    pub fn radius_accept(
        &mut self,
        leases: &lease::Table<'_>,
        attempt: Attempt,
        seconds: u32,
    ) -> Result<(), Error> {
        self.authorize(leases, attempt, seconds, Provider::Radius)
    }

    /// Fail the current external attempt without queuing policy application.
    pub fn radius_deny(
        &mut self,
        leases: &lease::Table<'_>,
        attempt: Attempt,
        reason: EndReason,
    ) -> Result<(), Error> {
        self.check_clock(leases)?;
        if !matches!(
            reason,
            EndReason::AuthenticationRejected | EndReason::AuthenticationFailed
        ) {
            return Err(Error::InvalidState);
        }
        let index = self.current_attempt(attempt, Phase::AwaitingAuthentication)?;
        self.slots[index].terminate(reason);
        Ok(())
    }

    fn current_attempt(&self, attempt: Attempt, phase: Phase) -> Result<usize, Error> {
        let index = self
            .index(attempt.session)
            .map_err(|_| Error::StaleAttempt)?;
        let slot = &self.slots[index];
        if slot.phase != phase || slot.attempt != Some(attempt) || self.now >= attempt.expires_at {
            return Err(Error::StaleAttempt);
        }
        Ok(index)
    }

    fn authorize(
        &mut self,
        leases: &lease::Table<'_>,
        attempt: Attempt,
        seconds: u32,
        provider: Provider,
    ) -> Result<(), Error> {
        self.check_clock(leases)?;
        let phase = match provider {
            Provider::Consent => Phase::AwaitingConsent,
            Provider::Radius => Phase::AwaitingAuthentication,
        };
        let index = self.current_attempt(attempt, phase)?;
        let slot = &self.slots[index];
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
        slot.provider = Some(provider);
        slot.pending = Some(EffectKind::Apply { until });
        Ok(())
    }
}
