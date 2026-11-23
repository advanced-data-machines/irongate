//! Single-owner live lab state; no per-packet heap storage or formatted logs.

use super::profile;
use irongate_core::{lease, session};

#[derive(Default)]
pub struct Counters {
    pub control_errors: u64,
    pub rx: u64,
    pub invalid: u64,
    pub denied: u64,
    pub tx_errors: u64,
    pub forwarded: u64,
    pub forwarded_ip_bytes: u64,
    pub arp: u64,
    pub offers: u64,
    pub acks: u64,
    pub naks: u64,
    pub applies: u64,
    pub removals: u64,
}

pub struct State<'a> {
    pub leases: lease::Table<'a>,
    pub sessions: session::Table<'a>,
    pub active: Option<session::Handle>,
    pub applied: Option<(session::Handle, lease::Handle, u64)>,
    pub counters: Counters,
    pub tokens: super::portal::Tokens,
    pub journal: super::portal::Journal,
}

impl<'a> State<'a> {
    pub fn new(lease_slots: &'a mut [lease::Slot], session_slots: &'a mut [session::Slot]) -> Self {
        let leases =
            lease::Table::new(profile::lease_config(), lease_slots).expect("fixed lab profile");
        let sessions = session::Table::new(profile::session_config(), &leases, session_slots)
            .expect("fixed session profile");
        Self {
            leases,
            sessions,
            active: None,
            applied: None,
            counters: Counters::default(),
            tokens: Default::default(),
            journal: Default::default(),
        }
    }

    pub fn advance(&mut self, now: u64) -> Result<(), String> {
        self.tokens.expire(now);
        let mut events = [None; 1];
        let count = self
            .leases
            .advance(now, &mut events)
            .map_err(|e| format!("lease clock: {e:?}"))?;
        self.sessions
            .advance(&self.leases)
            .map_err(|e| format!("session clock: {e:?}"))?;
        for event in events[..count].iter().flatten() {
            if event.kind == lease::EventKind::BindingExpired {
                self.sessions.lease_lost(event.handle);
            }
        }
        self.pump()
    }

    pub fn pump(&mut self) -> Result<(), String> {
        // The worker is this owner's actual fixed policy cell, not a kernel rule.
        // Packet transit checks both this cell and the current session decision.
        for _ in 0..4 {
            let Some(effect) = self
                .sessions
                .take_effect(&self.leases)
                .map_err(|e| format!("dispatch: {e:?}"))?
            else {
                return Ok(());
            };
            match effect.kind {
                session::EffectKind::Apply { until } => {
                    self.tokens.invalidate();
                    self.applied = Some((effect.token.session(), effect.lease, until));
                    self.counters.applies = self.counters.applies.saturating_add(1);
                }
                session::EffectKind::Remove { reason } => {
                    self.tokens.invalidate();
                    let kind = match reason {
                        session::EndReason::AuthorizationExpired
                        | session::EndReason::ConsentExpired => {
                            super::portal::journal::Kind::Expired
                        }
                        session::EndReason::LeaseLost | session::EndReason::LeaseExpired => {
                            super::portal::journal::Kind::LeaseLost
                        }
                        _ => super::portal::journal::Kind::Revoked,
                    };
                    self.journal
                        .stop(self.sessions.now(), self.counters.forwarded_ip_bytes, kind);
                    if self.applied.is_some_and(|(handle, lease, _)| {
                        handle == effect.token.session() && lease == effect.lease
                    }) {
                        self.applied = None;
                    }
                    self.counters.removals = self.counters.removals.saturating_add(1);
                }
            }
            self.sessions
                .complete(&self.leases, effect.token, true)
                .map_err(|e| format!("policy ack: {e:?}"))?;
        }
        Err("policy dispatch exceeded the one-session work bound".into())
    }

    pub fn allowed(&self) -> bool {
        self.applied.is_some_and(|(handle, lease, until)| {
            self.active == Some(handle)
                && self
                    .sessions
                    .view(handle)
                    .is_some_and(|view| view.lease == lease)
                && self.sessions.now() < until
                && self.sessions.allows(&self.leases, handle)
        })
    }

    pub fn attach(&mut self, lease: lease::Handle) -> Option<()> {
        if let Some(old) = self.active {
            let view = self.sessions.view(old)?;
            if view.lease == lease {
                return Some(());
            }
            self.tokens.invalidate();
            self.sessions.lease_lost(view.lease);
            self.pump().ok()?;
            self.sessions.forget(old).ok()?;
            self.active = None;
        }
        self.active = Some(self.sessions.attach(&self.leases, lease).ok()?);
        Some(())
    }
}
