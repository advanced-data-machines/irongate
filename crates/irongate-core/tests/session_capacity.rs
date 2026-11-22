//! Full storage retains cleanup; old generations cannot control reused addresses.
mod common;
#[path = "support/session.rs"]
mod support;

use irongate_core::{lease, session::*};

#[test]
fn full_session_storage_never_evicts_existing_access_or_drops_cleanup() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grants = [
        common::bind(&mut leases, &common::key(1), 1),
        common::bind(&mut leases, &common::key(2), 1),
        common::bind(&mut leases, &common::key(3), 1),
    ];
    let mut slots = [Slot::EMPTY; 2];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let one = sessions.attach(&leases, grants[0].handle).unwrap();
    let two = sessions.attach(&leases, grants[1].handle).unwrap();
    support::authorize(&mut sessions, &leases, one, 60);
    let stale_attempt = sessions.begin_consent(&leases, two).unwrap();
    assert_eq!(
        sessions.attach(&leases, grants[2].handle),
        Err(Error::Exhausted)
    );
    assert!(sessions.allows(&leases, one));
    sessions.logout(two).unwrap();
    sessions.forget(two).unwrap();
    let replacement = sessions.attach(&leases, grants[2].handle).unwrap();
    assert_ne!(replacement, two);
    assert_ne!(replacement.generation(), two.generation());
    assert_eq!(
        sessions.consent(&leases, stale_attempt, 60),
        Err(Error::StaleAttempt)
    );
    assert_eq!(sessions.logout(two), Err(Error::StaleHandle));
    sessions.logout(one).unwrap();
    let removal = sessions.take_effect(&leases).unwrap().unwrap();
    sessions.complete(&leases, removal.token, false).unwrap();
    assert_eq!(sessions.forget(one), Err(Error::InvalidState));
    assert_eq!(
        sessions.attach(&leases, grants[1].handle),
        Err(Error::Exhausted)
    );
    support::remove(&mut sessions, &leases);
    sessions.forget(one).unwrap();
    sessions.attach(&leases, grants[1].handle).unwrap();
}

#[test]
fn reused_address_waits_for_old_in_flight_grant_and_confirmed_removal() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let client = common::key(1);
    let grant = common::bind(&mut leases, &client, 1);
    let mut slots = [Slot::EMPTY; 2];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let old = sessions.attach(&leases, grant.handle).unwrap();
    let apply = support::apply(&mut sessions, &leases, old, 60);
    leases
        .apply(lease::Command::Release {
            client: &client,
            address: grant.address,
            server: common::config().server,
        })
        .unwrap();
    assert!(!sessions.allows(&leases, old));
    assert_eq!(sessions.lease_lost(grant.handle), 1);
    let next_client = common::key(2);
    let lease::Outcome::Offer(next) = leases
        .apply(lease::Command::Discover {
            client: &next_client,
            xid: 2,
            requested: Some(grant.address),
        })
        .unwrap()
    else {
        panic!()
    };
    common::select(&mut leases, &next_client, 2, next.address);
    assert_ne!(next.handle, grant.handle);
    assert_eq!(
        sessions.attach(&leases, next.handle),
        Err(Error::AddressBusy)
    );
    sessions.complete(&leases, apply.token, true).unwrap();
    assert_eq!(
        sessions.attach(&leases, next.handle),
        Err(Error::AddressBusy)
    );
    support::remove(&mut sessions, &leases);
    sessions.forget(old).unwrap();
    let current = sessions.attach(&leases, next.handle).unwrap();
    support::authorize(&mut sessions, &leases, current, 60);
    assert_eq!(
        sessions.complete(&leases, apply.token, true),
        Err(Error::StaleEffect)
    );
    assert_eq!(sessions.lease_lost(grant.handle), 0);
    assert!(sessions.allows(&leases, current));
}

#[test]
fn release_before_grant_dispatch_cancels_it_even_without_a_separate_notification() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let client = common::key(1);
    let grant = common::bind(&mut leases, &client, 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let attempt = sessions.begin_consent(&leases, handle).unwrap();
    sessions.consent(&leases, attempt, 60).unwrap();
    leases
        .apply(lease::Command::Release {
            client: &client,
            address: grant.address,
            server: common::config().server,
        })
        .unwrap();
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    assert_eq!(
        sessions.view(handle).unwrap().end_reason,
        Some(EndReason::LeaseLost)
    );
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoked);
}

#[test]
fn exact_pilot_limit_retains_an_effect_cell_for_every_admitted_session() {
    let config = lease::PoolConfig {
        prefix: 16,
        first: [10, 0, 0, 2],
        last: [10, 0, 1, 1],
        ..common::config()
    };
    let mut lease_slots = vec![lease::Slot::EMPTY; MAX_SESSIONS];
    let mut leases = lease::Table::new(config, &mut lease_slots).unwrap();
    let mut slots = vec![Slot::EMPTY; MAX_SESSIONS];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let mut handles = Vec::new();
    for id in 0..MAX_SESSIONS {
        let grant = common::bind(&mut leases, &common::key(id as u8), 1);
        let handle = sessions.attach(&leases, grant.handle).unwrap();
        support::authorize(&mut sessions, &leases, handle, 60);
        handles.push(handle);
    }
    assert_eq!(sessions.capacity(), MAX_SESSIONS);
    for handle in &handles {
        sessions.logout(*handle).unwrap();
    }
    for _ in 0..MAX_SESSIONS {
        support::remove(&mut sessions, &leases);
    }
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    assert!(
        handles
            .iter()
            .all(|handle| sessions.view(*handle).unwrap().phase == Phase::Revoked)
    );
}
