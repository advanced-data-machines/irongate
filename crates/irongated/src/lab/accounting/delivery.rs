//! Only an exact in-flight receipt may acknowledge or release retained storage.
use super::{Outbox, model::*};
impl Outbox {
    pub fn disconnected(&mut self) {
        if self.is_enabled() {
            self.summary.unavailable = true;
        }
    }
    pub fn complete(&mut self, receipt: Receipt, now: u64) {
        let Some(flight) = self.flight.filter(|f| f.dispatch.token == receipt.token) else {
            self.summary.stale = self.summary.stale.saturating_add(1);
            return;
        };
        self.flight = None;
        self.summary.stalled = false;
        self.summary.invalid = self
            .summary
            .invalid
            .saturating_add(u64::from(receipt.invalid));
        let index = receipt.token.slot;
        let Some(life) = self.lives[index]
            .as_mut()
            .filter(|l| l.serial == receipt.token.serial)
        else {
            return;
        };
        if receipt.outcome == Outcome::Ack {
            self.summary.unavailable = false;
            self.summary.outage_since = None;
            self.summary.last_acknowledged_id = Some(flight.dispatch.record.session_id);
            self.summary.last_acknowledged_kind = Some(flight.dispatch.record.kind);
            self.summary.last_response = Some(now);
            self.summary.acknowledged = self.summary.acknowledged.saturating_add(1);
            self.summary.last_acknowledged = Some(flight.dispatch.record.octets);
            life.failures = 0;
            life.retry_at = now;
            match flight.dispatch.record.kind {
                Kind::Start => life.start_acked = true,
                Kind::Interim => {
                    if life
                        .interim
                        .is_some_and(|r| r.created <= flight.dispatch.record.created)
                    {
                        life.interim = None;
                    }
                }
                Kind::Stop => {
                    self.lives[index] = None;
                }
            }
        } else {
            self.summary.unavailable = true;
            self.summary.outage_since.get_or_insert(now);
            self.summary.failures = self.summary.failures.saturating_add(1);
            life.failures = life.failures.saturating_add(1);
            life.retry_at = now.saturating_add(1u64 << life.failures.min(3));
            // Ended sessions need only their final cumulative Stop after Start.
            if life.stop.is_some() {
                life.interim = None;
            }
        }
    }
}
