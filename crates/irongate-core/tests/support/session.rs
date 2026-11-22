//! Synthetic policy worker helpers; no sockets or platform enforcement.
#![allow(dead_code)]

use irongate_core::{lease, session::*};

pub fn config() -> Config {
    Config {
        network: lease::NetworkId(1),
        epoch: 7,
        consent_seconds: 10,
        max_authorized_seconds: 300,
    }
}

pub fn tick(leases: &mut lease::Table<'_>, sessions: &mut Table<'_>, now: u64) {
    let mut events = [None; lease::MAX_LEASES];
    let count = leases.advance(now, &mut events).unwrap();
    sessions.advance(leases).unwrap();
    for event in events[..count].iter().flatten() {
        if event.kind == lease::EventKind::BindingExpired {
            sessions.lease_lost(event.handle);
        }
    }
}

pub fn apply(
    sessions: &mut Table<'_>,
    leases: &lease::Table<'_>,
    handle: Handle,
    seconds: u32,
) -> Effect {
    let attempt = sessions.begin_consent(leases, handle).unwrap();
    sessions.consent(leases, attempt, seconds).unwrap();
    assert!(!sessions.allows(leases, handle));
    let effect = sessions.take_effect(leases).unwrap().unwrap();
    assert_eq!(effect.token.session(), handle);
    assert!(matches!(effect.kind, EffectKind::Apply { .. }));
    assert!(!sessions.allows(leases, handle));
    effect
}

pub fn authorize(
    sessions: &mut Table<'_>,
    leases: &lease::Table<'_>,
    handle: Handle,
    seconds: u32,
) -> Effect {
    let effect = apply(sessions, leases, handle, seconds);
    sessions.complete(leases, effect.token, true).unwrap();
    assert!(sessions.allows(leases, handle));
    effect
}

pub fn remove(sessions: &mut Table<'_>, leases: &lease::Table<'_>) -> Effect {
    let effect = sessions.take_effect(leases).unwrap().unwrap();
    assert!(matches!(effect.kind, EffectKind::Remove { .. }));
    sessions.complete(leases, effect.token, true).unwrap();
    effect
}
