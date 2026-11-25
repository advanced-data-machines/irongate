//! Timer-driven cumulative updates coalesce; there is never a catch-up flood.
use super::{Outbox, model::*};
impl Outbox {
    pub fn tick(&mut self, now: u64) {
        if let Some(index) = self.active
            && let Some(life) = self.lives[index].as_mut()
            && life.activated
            && now >= life.next_interim
        {
            life.interim = Some(life.snapshot(Kind::Interim, now, 0));
            life.next_interim = now.saturating_add(self.interval);
        }
        self.summary.stalled = self.flight.is_some_and(|f| now.saturating_sub(f.sent) >= 3);
    }
    pub fn next(&mut self, now: u64) -> Option<Dispatch> {
        if self.flight.is_some() {
            return None;
        }
        let (index, life) = self
            .lives
            .iter()
            .enumerate()
            .filter_map(|(i, l)| Some((i, l.as_ref()?)))
            .filter(|(_, l)| l.retry_at <= now && l.pending().is_some())
            .min_by_key(|(_, l)| l.serial)?;
        let record = life.pending()?;
        let sequence = self.sequence.checked_add(1)?;
        self.sequence = sequence;
        let token = Token {
            slot: index,
            serial: life.serial,
            sequence,
        };
        let delay = now.saturating_sub(record.created).min(u64::from(u32::MAX)) as u32;
        let dispatch = Dispatch {
            token,
            record,
            delay,
        };
        self.flight = Some(Flight {
            dispatch,
            sent: now,
        });
        self.summary.sends = self.summary.sends.saturating_add(1);
        self.summary.last_exported_id = Some(record.session_id);
        self.summary.last_exported_kind = Some(record.kind);
        self.summary.last_exported = Some(record.octets);
        self.summary.last_send_lag = delay;
        Some(dispatch)
    }
}
