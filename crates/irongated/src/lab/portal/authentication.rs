//! A validated worker decision can only complete the current external attempt.
use super::bridge::Reply;
use crate::lab::{
    radius::{Decision, Outcome},
    state::State,
};
use irongate_core::{lease, session};
impl State<'_> {
    pub(super) fn begin_radius(&mut self, handle: session::Handle) -> Reply {
        if !self.journal.can_admit() || !self.accounting.can_admit() {
            return Reply::Error(503, "Local records or accounting capacity are unavailable.");
        }
        match self.sessions.begin_radius(&self.leases, handle) {
            Ok(attempt) => {
                self.authentication.attempts = self.authentication.attempts.saturating_add(1);
                Reply::Authenticate { attempt }
            }
            Err(_) => Reply::Error(409, "Session changed. Reload the portal."),
        }
    }
    pub(super) fn radius_result(
        &mut self,
        attempt: session::Attempt,
        decision: Decision,
        identity: Option<crate::lab::accounting::UserName>,
    ) -> Reply {
        let Some(view) = self.sessions.view(attempt.session()) else {
            return self.stale_radius();
        };
        if self.active != Some(view.handle)
            || !self.leases.view(view.lease).is_some_and(|l| {
                l.phase == lease::Phase::Bound && l.expires_at > self.sessions.now()
            })
            || self.provider != session::Provider::Radius
            || view.attempt != Some(attempt)
            || view.phase != session::Phase::AwaitingAuthentication
            || self.sessions.now() >= attempt.expires_at()
        {
            return self.stale_radius();
        }
        self.authentication.observe(decision, self.sessions.now());
        self.tokens.invalidate();
        match decision.outcome {
            Outcome::Accept { seconds } => {
                if !self.journal.can_admit() || !self.accounting.can_admit() {
                    let _ = self.sessions.radius_deny(
                        &self.leases,
                        attempt,
                        session::EndReason::AuthenticationFailed,
                    );
                    return Reply::Error(
                        503,
                        "Local records or accounting capacity are unavailable.",
                    );
                }
                if self.accounting.is_enabled()
                    && !identity.is_some_and(|user| {
                        self.accounting
                            .reserve(view.handle, view.lease, user, self.sessions.now())
                    })
                {
                    let _ = self.sessions.radius_deny(
                        &self.leases,
                        attempt,
                        session::EndReason::AuthenticationFailed,
                    );
                    return Reply::Error(503, "Accounting identity or capacity unavailable.");
                }
                if self
                    .sessions
                    .radius_accept(&self.leases, attempt, seconds)
                    .is_err()
                {
                    self.accounting.cancel();
                    return self.stale_radius();
                }
                self.finish_admission(view.handle, view.lease)
            }
            outcome => {
                let reason = if outcome == Outcome::Reject {
                    session::EndReason::AuthenticationRejected
                } else {
                    session::EndReason::AuthenticationFailed
                };
                if self
                    .sessions
                    .radius_deny(&self.leases, attempt, reason)
                    .is_err()
                {
                    return self.stale_radius();
                }
                let (code, message) = match outcome {
                    Outcome::Reject => (
                        401,
                        "Sign-in was rejected. Check your credentials or contact the network operator.",
                    ),
                    Outcome::Unsupported => (
                        422,
                        "The backend requested an unsupported authentication step or access policy.",
                    ),
                    Outcome::Invalid => (
                        502,
                        "The backend response could not be validated. Contact the network operator.",
                    ),
                    _ => (
                        503,
                        "The authentication service is unavailable. Please retry.",
                    ),
                };
                Reply::Error(code, message)
            }
        }
    }
    fn stale_radius(&mut self) -> Reply {
        self.authentication.stale = self.authentication.stale.saturating_add(1);
        Reply::Error(
            409,
            "Authentication attempt changed or expired. Reload the portal.",
        )
    }
    pub(super) fn finish_admission(
        &mut self,
        handle: session::Handle,
        lease: lease::Handle,
    ) -> Reply {
        if self.pump().is_err() || !self.allowed() {
            self.applied = None;
            self.accounting.cancel();
            return Reply::Error(503, "Access could not be applied. Please retry.");
        }
        self.accounting.activate(handle, lease, self.sessions.now());
        let provider = self
            .sessions
            .view(handle)
            .and_then(|v| v.provider)
            .expect("applied policy source");
        assert!(
            self.journal.grant(
                handle,
                lease,
                self.sessions.now(),
                self.counters.forwarded_ip_bytes,
                provider
            ),
            "single owner checked lifecycle record capacity"
        );
        Reply::Redirect
    }
}
