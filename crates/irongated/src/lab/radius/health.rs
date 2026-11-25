//! Fixed observations for authentication only; no inferred accounting health.
use super::{Decision, Outcome};
#[derive(Clone, Copy, Default)]
pub(crate) struct Health {
    pub attempts: u64,
    pub accepted: u64,
    pub rejected: u64,
    pub timed_out: u64,
    pub invalid: u64,
    pub unavailable: u64,
    pub unsupported: u64,
    pub stale: u64,
    pub last: Option<Decision>,
    pub validated_at: Option<u64>,
}
impl Health {
    pub fn observe(&mut self, decision: Decision, now: u64) {
        self.invalid = self.invalid.saturating_add(u64::from(decision.invalid));
        match decision.outcome {
            Outcome::Accept { .. } => {
                self.accepted = self.accepted.saturating_add(1);
                self.validated_at = Some(now);
            }
            Outcome::Reject => {
                self.rejected = self.rejected.saturating_add(1);
                self.validated_at = Some(now);
            }
            Outcome::Unsupported => {
                self.unsupported = self.unsupported.saturating_add(1);
                self.validated_at = Some(now);
            }
            Outcome::Timeout => self.timed_out = self.timed_out.saturating_add(1),
            Outcome::Invalid => (),
            Outcome::Unavailable => self.unavailable = self.unavailable.saturating_add(1),
        }
        self.last = Some(decision);
    }
}
impl Health {
    pub fn status(self, enabled: bool, now: u64) -> serde_json::Value {
        use serde_json::json;
        if !enabled {
            return json!({"health":"disabled","accounting":"not_implemented"});
        }
        let (health, outcome) = match self.last.map(|d| d.outcome) {
            None => ("unknown", "not_observed"),
            Some(Outcome::Accept { .. }) => ("responsive", "access_accept"),
            Some(Outcome::Reject) => ("responsive", "access_reject"),
            Some(Outcome::Unsupported) => ("responsive", "unsupported_policy_or_challenge"),
            Some(Outcome::Timeout) => ("unavailable", "timeout"),
            Some(Outcome::Invalid) => ("unavailable", "invalid_response"),
            Some(Outcome::Unavailable) => ("unavailable", "transport_failure"),
        };
        json!({"health":health,"last_outcome":outcome,"endpoint":"192.0.2.80:1812","nas_ip":"192.0.2.1",
            "validated_at":self.validated_at,"validated_age_seconds":self.validated_at.map(|at|now.saturating_sub(at)),"elapsed_ms":self.last.map(|d|d.elapsed_ms),"last_sends":self.last.map(|d|d.sends),
            "attempts":self.attempts,"access_accepts":self.accepted,"access_rejects":self.rejected,"timeouts":self.timed_out,
            "invalid_responses":self.invalid,"transport_failures":self.unavailable,"unsupported":self.unsupported,"stale_results":self.stale,
            "accounting":"not_implemented"})
    }
}
