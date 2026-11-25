//! Fixed storage, trusted lease-arena binding and bounded identity lookup.

use super::{Config, EffectToken, Error, Handle, MAX_SESSIONS, Slot, View};
use crate::lease;

/// Single owner of one network's consent/admission state and policy effects.
pub struct Table<'a> {
    pub(super) config: Config,
    pub(super) lease_epoch: u64,
    pub(super) slots: &'a mut [Slot],
    pub(super) now: u64,
    pub(super) sequence: u64,
    pub(super) dispatch_cursor: usize,
}

impl<'a> Table<'a> {
    /// Bind empty caller-owned storage to one authoritative lease arena.
    /// Every reconstruction/reload requires a distinct nonzero session epoch.
    pub fn new(
        config: Config,
        leases: &lease::Table<'_>,
        slots: &'a mut [Slot],
    ) -> Result<Self, Error> {
        if config.epoch == 0
            || config.consent_seconds == 0
            || config.max_authorized_seconds == 0
            || slots.is_empty()
            || slots.len() > MAX_SESSIONS
        {
            return Err(Error::InvalidConfig);
        }
        if config.network != leases.network() {
            return Err(Error::WrongLeaseTable);
        }
        if slots.iter().any(|slot| slot.lease.is_some()) {
            return Err(Error::InvalidStorage);
        }
        Ok(Self {
            config,
            lease_epoch: leases.epoch(),
            slots,
            now: leases.now(),
            sequence: 0,
            dispatch_cursor: 0,
        })
    }

    /// Fixed session capacity, including slots retained during cleanup/diagnosis.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    /// Last successful explicit advance, in monotonic seconds.
    pub fn now(&self) -> u64 {
        self.now
    }

    /// Attach a currently bound lease without granting access. Duplicate attachment
    /// returns its existing session; closed sessions require a new consent attempt.
    pub fn attach(
        &mut self,
        leases: &lease::Table<'_>,
        lease: lease::Handle,
    ) -> Result<Handle, Error> {
        self.check_clock(leases)?;
        let view = self.bound_lease(leases, lease)?;
        if let Some(index) = self.slots.iter().position(|slot| slot.lease == Some(lease)) {
            return Ok(self.handle(index));
        }
        if self
            .slots
            .iter()
            .any(|slot| slot.lease.is_some() && slot.address == view.address && !slot.closed())
        {
            return Err(Error::AddressBusy);
        }
        for index in 0..self.slots.len() {
            let slot = &mut self.slots[index];
            if slot.lease.is_some() || slot.retired {
                continue;
            }
            let Some(generation) = slot.generation.checked_add(1) else {
                slot.retired = true;
                continue;
            };
            *slot = Slot {
                generation,
                lease: Some(lease),
                address: view.address,
                lease_until: view.expires_at,
                ..Slot::EMPTY
            };
            return Ok(self.handle(index));
        }
        Err(Error::Exhausted)
    }

    /// Explicitly release diagnostic storage only after all cleanup is confirmed.
    pub fn forget(&mut self, handle: Handle) -> Result<(), Error> {
        let index = self.index(handle)?;
        if !self.slots[index].closed() {
            return Err(Error::InvalidState);
        }
        self.slots[index].clear();
        Ok(())
    }

    /// Last observed state; this snapshot alone is not an access decision.
    pub fn view(&self, handle: Handle) -> Option<View> {
        let index = self.index(handle).ok()?;
        let slot = &self.slots[index];
        Some(View {
            handle,
            lease: slot.lease?,
            address: slot.address,
            phase: slot.phase,
            provider: slot.provider,
            lease_until: slot.lease_until,
            authorized_until: slot.authorized_until,
            attempt: slot.attempt,
            pending: slot.pending,
            in_flight: slot.in_flight,
            end_reason: slot.end_reason,
            removal_failures: slot.removal_failures,
        })
    }

    /// Fail closed on stale identity, unobserved time, missing lease, pending policy
    /// or expired authorization. The owner must supply a current monotonic clock.
    pub fn allows(&self, leases: &lease::Table<'_>, handle: Handle) -> bool {
        if self.check_clock(leases).is_err() {
            return false;
        }
        let Ok(index) = self.index(handle) else {
            return false;
        };
        let slot = &self.slots[index];
        slot.phase == super::Phase::Authorized
            && slot.authorized_until.is_some_and(|until| self.now < until)
            && slot
                .lease
                .is_some_and(|lease| self.bound_lease(leases, lease).is_ok())
    }

    pub(super) fn check_table(&self, leases: &lease::Table<'_>) -> Result<(), Error> {
        if leases.network() != self.config.network || leases.epoch() != self.lease_epoch {
            return Err(Error::WrongLeaseTable);
        }
        Ok(())
    }
    pub(super) fn check_clock(&self, leases: &lease::Table<'_>) -> Result<(), Error> {
        self.check_table(leases)?;
        if leases.now() != self.now {
            return Err(Error::ClockMismatch);
        }
        Ok(())
    }
    pub(super) fn bound_lease<'b>(
        &self,
        leases: &'b lease::Table<'_>,
        handle: lease::Handle,
    ) -> Result<lease::View<'b>, Error> {
        let view = leases.view(handle).ok_or(Error::InvalidLease)?;
        if view.phase != lease::Phase::Bound || view.expires_at <= self.now {
            return Err(Error::InvalidLease);
        }
        Ok(view)
    }
    pub(super) fn index(&self, handle: Handle) -> Result<usize, Error> {
        if handle.network == self.config.network && handle.epoch == self.config.epoch {
            let index = usize::from(handle.index);
            if self
                .slots
                .get(index)
                .is_some_and(|slot| slot.lease.is_some() && slot.generation == handle.generation)
            {
                return Ok(index);
            }
        }
        Err(Error::StaleHandle)
    }
    pub(super) fn handle(&self, index: usize) -> Handle {
        Handle {
            network: self.config.network,
            epoch: self.config.epoch,
            index: index as u16,
            generation: self.slots[index].generation,
        }
    }
    pub(super) fn next_sequence(&mut self) -> Result<u64, Error> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        Ok(self.sequence)
    }
    pub(super) fn effect_index(&self, token: EffectToken) -> Result<usize, Error> {
        let index = self.index(token.session).map_err(|_| Error::StaleEffect)?;
        if self.slots[index]
            .in_flight
            .is_some_and(|effect| effect.token == token)
        {
            Ok(index)
        } else {
            Err(Error::StaleEffect)
        }
    }
}
