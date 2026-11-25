//! Bounded metadata copied at the owner boundary; formatting stays on workers.
use super::{Outbox, model::Octets, outbox::CAPACITY};
#[derive(Clone, Copy, Default)]
pub(crate) struct Summary {
    pub enabled: bool,
    pub interim_seconds: u64,
    pub unavailable: bool,
    pub stalled: bool,
    pub retained: usize,
    pub pending: usize,
    pub in_flight: bool,
    pub oldest_pending: u64,
    pub sends: u64,
    pub acknowledged: u64,
    pub failures: u64,
    pub invalid: u64,
    pub stale: u64,
    pub last_response: Option<u64>,
    pub last_send_lag: u32,
    pub outage_since: Option<u64>,
    pub current_id: Option<[u8; 58]>,
    pub last_exported_id: Option<[u8; 58]>,
    pub last_acknowledged_id: Option<[u8; 58]>,
    pub last_exported_kind: Option<super::model::Kind>,
    pub last_acknowledged_kind: Option<super::model::Kind>,
    pub current: Option<Octets>,
    pub last_exported: Option<Octets>,
    pub last_acknowledged: Option<Octets>,
}
impl Outbox {
    pub fn snapshot(&self, now: u64) -> Summary {
        let mut summary = self.summary;
        summary.enabled = self.is_enabled();
        summary.interim_seconds = self.interval;
        summary.in_flight = self.flight.is_some();
        summary.current_id = self
            .active
            .and_then(|i| self.lives[i].map(|l| l.start.session_id));
        summary.current = self.active.and_then(|i| self.lives[i].map(|l| l.totals));
        for life in self.lives.iter().flatten() {
            summary.retained += 1;
            summary.pending += usize::from(!life.start_acked)
                + usize::from(life.interim.is_some())
                + usize::from(life.stop.is_some());
            if let Some(record) = life.pending() {
                summary.oldest_pending = summary
                    .oldest_pending
                    .max(now.saturating_sub(record.created));
            }
        }
        summary
    }
}
impl Summary {
    pub fn json(self, now: u64) -> serde_json::Value {
        use serde_json::json;
        let octets = |o: Octets| json!({"input":o.input,"output":o.output,"overflow":o.overflow});
        let id = |id: [u8; 58]| String::from_utf8_lossy(&id).into_owned();
        let kind = |kind| match kind {
            super::model::Kind::Start => "start",
            super::model::Kind::Interim => "interim",
            super::model::Kind::Stop => "stop",
        };
        json!({"enabled":self.enabled,"state":if !self.enabled{"disabled"}else if self.stalled{"stalled"}else if self.unavailable{"unavailable"}else if self.last_response.is_some(){"responsive"}else{"unknown"},
            "interim_seconds":self.interim_seconds,"storage":"memory","crash_replay":false,"capacity_lifecycles":CAPACITY,"retained_lifecycles":self.retained,
            "pending":self.pending,"outage_seconds":self.outage_since.map(|t|now.saturating_sub(t)),
            "memory_bytes":std::mem::size_of::<Outbox>(),"retry_backoff_seconds":[2,4,8],"interim_policy":"latest_cumulative_snapshot",
            "current_session_id":self.current_id.map(id),"last_exported_session_id":self.last_exported_id.map(id),"last_acknowledged_session_id":self.last_acknowledged_id.map(id),
            "last_exported_kind":self.last_exported_kind.map(kind),"last_acknowledged_kind":self.last_acknowledged_kind.map(kind),"in_flight":self.in_flight,"oldest_pending_seconds":self.oldest_pending,
            "sends":self.sends,"acknowledged":self.acknowledged,"failed_attempts":self.failures,"invalid_responses":self.invalid,"stale_receipts":self.stale,
            "last_response_age_seconds":self.last_response.map(|t|now.saturating_sub(t)),"last_send_lag_seconds":self.last_send_lag,
            "current_octets":self.current.map(octets),"last_exported_octets":self.last_exported.map(octets),"last_acknowledged_octets":self.last_acknowledged.map(octets),
            "count_point":"successfully_submitted_ip_octets","input_direction":"subscriber_to_upstream","delivery":"unverified","backend_billing":"unverified"})
    }
}
