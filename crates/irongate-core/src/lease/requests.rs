//! SELECTING versus INIT-REBOOT/renewal have different evidence requirements.

use super::model::State;
use super::{ClientKey, Error, IgnoreReason, NakReason, Outcome, RequestMode, Table};

impl Table<'_> {
    pub(super) fn request(
        &mut self,
        client: &ClientKey,
        xid: u32,
        mode: RequestMode,
    ) -> Result<Outcome, Error> {
        match mode {
            RequestMode::Selecting { server, address } => {
                if server != self.config.server {
                    // A delayed selection of another server must not cancel a
                    // newer transaction or discard a still-valid bound lease.
                    if let Some(index) = self.find(client) {
                        let slot = &mut self.slots[index];
                        if slot.pending.is_some_and(|pending| pending.xid == xid) {
                            if slot.state == State::Offered {
                                slot.clear();
                            } else {
                                slot.pending = None;
                            }
                        }
                    }
                    return Ok(Outcome::Ignore(IgnoreReason::OtherServer));
                }
                let Some(index) = self.index(address) else {
                    return Ok(Outcome::Nak(NakReason::WrongAddress));
                };
                let slot = &self.slots[index];
                if slot.client.as_ref() != Some(client) {
                    return Ok(Outcome::Nak(NakReason::NoOffer));
                }
                if slot.pending.is_some_and(|pending| pending.xid == xid) {
                    return self.bind(index, xid);
                }
                if slot.state == State::Bound && slot.last_xid == xid {
                    let remaining = (slot.expires - self.now) as u32;
                    return Ok(Outcome::Ack(self.grant(index, remaining)));
                }
                Ok(Outcome::Nak(NakReason::NoOffer))
            }
            RequestMode::InitReboot { address } | RequestMode::Renew { address } => {
                if !self.config.host(address) {
                    return Ok(Outcome::Nak(NakReason::WrongAddress));
                }
                let Some(index) = self.find(client) else {
                    return Ok(Outcome::Ignore(IgnoreReason::NoBinding));
                };
                if self.slots[index].state != State::Bound {
                    return Ok(Outcome::Ignore(IgnoreReason::NoBinding));
                }
                if self.address(index) != address {
                    return Ok(Outcome::Nak(NakReason::BindingMismatch));
                }
                self.bind(index, xid)
            }
        }
    }

    fn bind(&mut self, index: usize, xid: u32) -> Result<Outcome, Error> {
        let expires = self.deadline(self.config.lease_seconds)?;
        let slot = &mut self.slots[index];
        slot.state = State::Bound;
        slot.expires = expires;
        slot.last_xid = xid;
        slot.pending = None;
        Ok(Outcome::Ack(self.grant(index, self.config.lease_seconds)))
    }
}
