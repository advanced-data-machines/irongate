//! No implicit expiration: return every due effect before making storage reusable.

use super::model::{Slot, State};
use super::{Error, Event, EventKind, Table};

fn due(slot: &Slot, now: u64) -> Option<EventKind> {
    if slot.expires <= now {
        return match slot.state {
            State::Offered => Some(EventKind::OfferExpired),
            State::Bound => Some(EventKind::BindingExpired),
            State::Declined => Some(EventKind::QuarantineEnded),
            State::Free | State::Retired => None,
        };
    }
    (slot.state == State::Bound && slot.pending.is_some_and(|pending| pending.expires <= now))
        .then_some(EventKind::PendingOfferExpired)
}

impl Table<'_> {
    /// Advance monotonic seconds and return every due effect in `events[..count]`.
    /// Consume these before applying commands/reusing addresses. On insufficient
    /// event capacity or clock regression, nothing (including clock) changes.
    /// Cells after the returned count are untouched; old contents are not events.
    pub fn advance(&mut self, now: u64, events: &mut [Option<Event>]) -> Result<usize, Error> {
        if now < self.now {
            return Err(Error::ClockRegressed);
        }
        let required = self
            .slots
            .iter()
            .filter(|slot| due(slot, now).is_some())
            .count();
        if events.len() < required {
            return Err(Error::EventCapacity { required });
        }
        self.now = now;
        let mut count = 0;
        for index in 0..self.slots.len() {
            if let Some(kind) = due(&self.slots[index], now) {
                events[count] = Some(Event {
                    handle: self.handle(index),
                    kind,
                });
                count += 1;
                if kind == EventKind::PendingOfferExpired {
                    self.slots[index].pending = None;
                } else {
                    self.slots[index].clear();
                }
            }
        }
        Ok(count)
    }
}
