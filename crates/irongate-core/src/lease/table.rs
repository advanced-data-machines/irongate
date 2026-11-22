//! Caller-owned fixed storage; address lookup is indexed, client lookup bounded.

use super::model::{Pending, State};
use super::{
    ClientKey, Command, Error, Grant, Handle, IgnoreReason, Outcome, Phase, PoolConfig, Slot, View,
};

/// One network's lease arena. Call `advance` with a current monotonic timestamp,
/// consume its effects, then apply a batch of commands at that timestamp.
pub struct Table<'a> {
    pub(super) config: PoolConfig,
    pub(super) slots: &'a mut [Slot],
    pub(super) now: u64,
    cursor: usize,
}

impl<'a> Table<'a> {
    /// Borrow exactly one initially empty/retired slot per managed address.
    /// The owner must give every reconstructed arena a distinct nonzero epoch.
    pub fn new(config: PoolConfig, slots: &'a mut [Slot]) -> Result<Self, Error> {
        let count = config.validate()?;
        if slots.len() != count
            || slots
                .iter()
                .any(|slot| !matches!(slot.state, State::Free | State::Retired))
        {
            return Err(Error::InvalidStorage);
        }
        Ok(Self {
            config,
            slots,
            now: 0,
            cursor: 0,
        })
    }

    /// Fixed slot/address capacity.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    /// Trusted network selected when this pool was constructed.
    pub fn network(&self) -> super::NetworkId {
        self.config.network
    }
    /// Owner-assigned arena identity; dependent owners must reject replacement pools.
    pub fn epoch(&self) -> u64 {
        self.config.epoch
    }
    /// Most recent successful explicit time advance; no OS clock is read.
    pub fn now(&self) -> u64 {
        self.now
    }

    /// Apply a decoded command using the table's current explicit clock.
    /// This changes lease ownership only, never portal/access authorization.
    pub fn apply(&mut self, command: Command<'_>) -> Result<Outcome, Error> {
        let client = match command {
            Command::Discover { client, .. }
            | Command::Request { client, .. }
            | Command::Release { client, .. }
            | Command::Decline { client, .. } => client,
        };
        if client.network != self.config.network {
            return Err(Error::WrongNetwork);
        }
        match command {
            Command::Discover {
                client,
                xid,
                requested,
            } => self.discover(client, xid, requested),
            Command::Request { client, xid, mode } => self.request(client, xid, mode),
            Command::Release {
                client,
                address,
                server,
            } => self.end(client, address, server, false),
            Command::Decline {
                client,
                address,
                server,
            } => self.end(client, address, server, true),
        }
    }

    /// Resolve only a live matching generation/epoch/network. A lease view is not
    /// an access grant; callers must distinguish Offered/Bound/Declined.
    pub fn view(&self, handle: Handle) -> Option<View<'_>> {
        if handle.network != self.config.network || handle.epoch != self.config.epoch {
            return None;
        }
        let index = usize::from(handle.index);
        let slot = self.slots.get(index)?;
        if slot.generation != handle.generation {
            return None;
        }
        let phase = match slot.state {
            State::Offered => Phase::Offered,
            State::Bound => Phase::Bound,
            State::Declined => Phase::Declined,
            State::Free | State::Retired => return None,
        };
        Some(View {
            handle,
            address: self.address(index),
            phase,
            client: slot.client.as_ref(),
            expires_at: slot.expires,
        })
    }

    pub(super) fn handle(&self, index: usize) -> Handle {
        Handle {
            network: self.config.network,
            epoch: self.config.epoch,
            index: index as u16,
            generation: self.slots[index].generation,
        }
    }
    pub(super) fn address(&self, index: usize) -> [u8; 4] {
        (u32::from_be_bytes(self.config.first) + index as u32).to_be_bytes()
    }
    pub(super) fn index(&self, address: [u8; 4]) -> Option<usize> {
        let index = u32::from_be_bytes(address)
            .checked_sub(u32::from_be_bytes(self.config.first))? as usize;
        (index < self.slots.len()).then_some(index)
    }
    pub(super) fn find(&self, client: &ClientKey) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.client.as_ref() == Some(client))
    }
    pub(super) fn deadline(&self, seconds: u32) -> Result<u64, Error> {
        self.now
            .checked_add(u64::from(seconds))
            .ok_or(Error::ClockOverflow)
    }
    pub(super) fn grant(&self, index: usize, seconds: u32) -> Grant {
        Grant {
            handle: self.handle(index),
            address: self.address(index),
            lease_seconds: seconds,
        }
    }

    fn discover(
        &mut self,
        client: &ClientKey,
        xid: u32,
        requested: Option<[u8; 4]>,
    ) -> Result<Outcome, Error> {
        let expires = self.deadline(self.config.offer_seconds)?;
        if let Some(index) = self.find(client) {
            let slot = &mut self.slots[index];
            if !slot.pending.is_some_and(|pending| pending.xid == xid) {
                let expires = if slot.state == State::Bound {
                    expires.min(slot.expires)
                } else {
                    expires
                };
                slot.pending = Some(Pending { xid, expires });
                if slot.state == State::Offered {
                    slot.expires = expires;
                }
            }
            let seconds = if slot.state == State::Bound {
                (slot.expires - self.now) as u32
            } else {
                self.config.lease_seconds
            };
            return Ok(Outcome::Offer(self.grant(index, seconds)));
        }
        let start = requested
            .and_then(|ip| self.index(ip))
            .unwrap_or(self.cursor);
        for offset in 0..self.slots.len() {
            let index = (start + offset) % self.slots.len();
            let slot = &mut self.slots[index];
            if slot.state != State::Free {
                continue;
            }
            let Some(generation) = slot.generation.checked_add(1) else {
                slot.state = State::Retired;
                continue;
            };
            *slot = Slot {
                generation,
                state: State::Offered,
                client: Some(*client),
                expires,
                pending: Some(Pending { xid, expires }),
                last_xid: 0,
            };
            self.cursor = (index + 1) % self.slots.len();
            return Ok(Outcome::Offer(self.grant(index, self.config.lease_seconds)));
        }
        Err(Error::Exhausted)
    }

    fn end(
        &mut self,
        client: &ClientKey,
        address: [u8; 4],
        server: [u8; 4],
        decline: bool,
    ) -> Result<Outcome, Error> {
        if server != self.config.server {
            return Ok(Outcome::Ignore(IgnoreReason::OtherServer));
        }
        let Some(index) = self.index(address) else {
            return Ok(Outcome::Ignore(IgnoreReason::NotOwner));
        };
        let slot = &self.slots[index];
        if slot.client.as_ref() != Some(client)
            || !(slot.state == State::Bound || decline && slot.state == State::Offered)
        {
            return Ok(Outcome::Ignore(IgnoreReason::NotOwner));
        }
        let deadline = if decline {
            self.deadline(self.config.decline_seconds)?
        } else {
            0
        };
        let handle = self.handle(index);
        let slot = &mut self.slots[index];
        slot.clear();
        if decline {
            slot.state = State::Declined;
            slot.expires = deadline;
        }
        Ok(if decline {
            Outcome::Declined(handle)
        } else {
            Outcome::Released(handle)
        })
    }
}
