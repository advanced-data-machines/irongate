//! Session-bound actions run only on the single packet owner.

use super::{
    bridge::{Reply, Request, Snapshot},
    journal::CAPACITY,
    tokens::Action,
};
use crate::lab::{profile, state::State};
use irongate_core::{lease, session};
use std::net::Ipv4Addr;

impl State<'_> {
    pub(crate) fn portal_request(&mut self, peer: Ipv4Addr, request: Request) -> Reply {
        if peer.octets() != profile::CLIENT_IP {
            return Reply::Error(403, "This network session is unavailable.");
        }
        if let Request::RadiusResult { attempt, decision } = request {
            return self.radius_result(attempt, decision);
        }
        let Some(handle) = self.active else {
            return Reply::Error(409, "Obtain a network address before opening the portal.");
        };
        let Some(view) = self.sessions.view(handle) else {
            return Reply::Error(409, "Session changed. Reload the portal.");
        };
        if !self
            .leases
            .view(view.lease)
            .is_some_and(|v| v.phase == lease::Phase::Bound)
        {
            return Reply::Error(409, "Network lease ended. Reconnect before continuing.");
        }
        match request {
            Request::RadiusResult { attempt, decision } => self.radius_result(attempt, decision),
            Request::Act { action, token } => self.portal_action(handle, view.lease, action, token),
            Request::Script => Reply::Script,
            Request::Status | Request::View { .. } => {
                let allowed = self.allowed();
                let action = if let Request::View { entropy } = request {
                    let action = if allowed || view.phase == session::Phase::AwaitingAuthentication
                    {
                        Action::Logout
                    } else {
                        if self.provider == session::Provider::Radius {
                            Action::Login
                        } else {
                            Action::Consent
                        }
                    };
                    let Some((token, until)) =
                        self.tokens
                            .issue(entropy, handle, view.lease, action, self.sessions.now())
                    else {
                        return Reply::Error(
                            429,
                            "Too many pending actions. Wait a minute and reload.",
                        );
                    };
                    Some((action, token, until))
                } else {
                    None
                };
                Reply::View(Snapshot {
                    transport: None,
                    provider: self.provider,
                    policy_source: view.provider,
                    authentication: self.authentication,
                    phase: phase(view.phase),
                    now: self.sessions.now(),
                    allowed,
                    generation: handle.generation(),
                    until: view.authorized_until,
                    action,
                    records: self.journal.used,
                    durable: self.journal.durable,
                    record_failures: self.journal.failed,
                    record_stalled: self.journal.stalled,
                    pending_age: self.journal.pending_age(self.sessions.now()),
                    record_capacity: CAPACITY,
                    ip_bytes: self.counters.forwarded_ip_bytes,
                    admission_available: self.journal.can_admit(),
                })
            }
        }
    }

    fn portal_action(
        &mut self,
        handle: session::Handle,
        lease: lease::Handle,
        action: Action,
        token: [u8; 32],
    ) -> Reply {
        if (action == Action::Consent && self.provider != session::Provider::Consent)
            || (action == Action::Login && self.provider != session::Provider::Radius)
        {
            return Reply::Error(403, "This authentication method is not enabled.");
        }
        if !self
            .tokens
            .consume(token, handle, lease, action, self.sessions.now())
        {
            return Reply::Error(
                409,
                "Action expired, changed or was already used. Reload the portal.",
            );
        }
        self.tokens.invalidate();
        match action {
            Action::Login => return self.begin_radius(handle),
            Action::Consent => {
                if !self.journal.can_admit() {
                    return Reply::Error(
                        503,
                        "The local activity log is full or unavailable. New access is paused.",
                    );
                }
                let result = self
                    .sessions
                    .begin_consent(&self.leases, handle)
                    .and_then(|attempt| self.sessions.consent(&self.leases, attempt, 30));
                if result.is_err() {
                    return Reply::Error(409, "Session changed. Reload the portal.");
                }
                return self.finish_admission(handle, lease);
            }
            Action::Logout => {
                if self.sessions.logout(handle).is_err() || self.pump().is_err() {
                    return Reply::Error(503, "Access cleanup is pending.");
                }
            }
        }
        Reply::Redirect
    }
}

fn phase(phase: session::Phase) -> &'static str {
    match phase {
        session::Phase::Leased => "Leased",
        session::Phase::AwaitingConsent => "AwaitingConsent",
        session::Phase::AwaitingAuthentication => "AwaitingAuthentication",
        session::Phase::Applying => "Applying",
        session::Phase::Authorized => "Authorized",
        session::Phase::Revoking => "Revoking",
        session::Phase::Revoked => "Revoked",
        session::Phase::Expired => "Expired",
    }
}
