//! A bound address is not authorization; policy acknowledgements are explicit.
mod common;
#[path = "support/session.rs"]
mod support;

use irongate_core::{lease, session::*};

#[test]
fn lease_and_consent_stay_denied_until_exact_policy_application_is_confirmed() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let offer = common::discover(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY; 2];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    assert_eq!(
        sessions.attach(&leases, offer.handle),
        Err(Error::InvalidLease)
    );
    common::select(&mut leases, &common::key(1), 1, offer.address);
    let handle = sessions.attach(&leases, offer.handle).unwrap();
    assert_eq!(sessions.attach(&leases, offer.handle).unwrap(), handle);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Leased);
    assert!(!sessions.allows(&leases, handle));
    let effect = support::apply(&mut sessions, &leases, handle, 60);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Applying);
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    sessions.complete(&leases, effect.token, true).unwrap();
    assert!(sessions.allows(&leases, handle));
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Authorized);
    assert_eq!(
        sessions.complete(&leases, effect.token, true),
        Err(Error::StaleEffect)
    );
    assert_eq!(sessions.forget(handle), Err(Error::InvalidState));
}

#[test]
fn logout_denies_immediately_but_reserves_storage_until_removal_is_confirmed() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY; 2];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    support::authorize(&mut sessions, &leases, handle, 60);
    sessions.logout(handle).unwrap();
    sessions.logout(handle).unwrap(); // Coalesced, one removal obligation.
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoking);
    assert_eq!(sessions.forget(handle), Err(Error::InvalidState));
    assert_eq!(
        sessions.begin_consent(&leases, handle),
        Err(Error::InvalidState)
    );
    support::remove(&mut sessions, &leases);
    let view = sessions.view(handle).unwrap();
    assert_eq!(view.phase, Phase::Revoked);
    assert_eq!(view.end_reason, Some(EndReason::Logout));
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    sessions.forget(handle).unwrap();
    assert!(sessions.view(handle).is_none());
}

#[test]
fn attempt_replacement_and_expiry_never_authorize_a_late_consent_result() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let old = sessions.begin_consent(&leases, handle).unwrap();
    let current = sessions.begin_consent(&leases, handle).unwrap();
    assert_ne!(old, current);
    assert_eq!(sessions.consent(&leases, old, 30), Err(Error::StaleAttempt));
    assert_eq!(
        sessions.consent(&leases, current, 0),
        Err(Error::InvalidDuration)
    );
    assert_eq!(
        sessions.consent(&leases, current, 301),
        Err(Error::InvalidDuration)
    );
    support::tick(&mut leases, &mut sessions, 10);
    assert_eq!(
        sessions.consent(&leases, current, 30),
        Err(Error::StaleAttempt)
    );
    let view = sessions.view(handle).unwrap();
    assert_eq!(view.phase, Phase::Expired);
    assert_eq!(view.end_reason, Some(EndReason::ConsentExpired));
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    // A new explicit attempt is required; the old result remains invalid.
    let next = sessions.begin_consent(&leases, handle).unwrap();
    assert_ne!(next, current);
    sessions.consent(&leases, next, 30).unwrap();
    assert!(!sessions.allows(&leases, handle));
}

#[test]
fn authorization_expiry_and_unobserved_clock_both_fail_closed() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    support::authorize(&mut sessions, &leases, handle, 30);
    common::advance(&mut leases, 29);
    assert!(!sessions.allows(&leases, handle)); // Session clock has not observed 29.
    sessions.advance(&leases).unwrap();
    assert!(sessions.allows(&leases, handle));
    support::tick(&mut leases, &mut sessions, 30);
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(
        sessions.view(handle).unwrap().end_reason,
        Some(EndReason::AuthorizationExpired)
    );
    support::remove(&mut sessions, &leases);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Expired);
}

#[test]
fn access_is_capped_by_lease_expiry_and_a_lease_renewal_does_not_extend_consent() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let client = common::key(1);
    let grant = common::bind(&mut leases, &client, 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let apply = support::authorize(&mut sessions, &leases, handle, 300);
    assert_eq!(apply.kind, EffectKind::Apply { until: 120 });
    support::tick(&mut leases, &mut sessions, 60);
    leases
        .apply(lease::Command::Request {
            client: &client,
            xid: 2,
            mode: lease::RequestMode::Renew {
                address: grant.address,
            },
        })
        .unwrap();
    sessions.advance(&leases).unwrap();
    assert_eq!(sessions.view(handle).unwrap().lease_until, 180);
    assert_eq!(sessions.view(handle).unwrap().authorized_until, Some(120));
    support::tick(&mut leases, &mut sessions, 120);
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(
        sessions.view(handle).unwrap().end_reason,
        Some(EndReason::AuthorizationExpired)
    );
}

#[test]
fn lease_expiry_is_observed_even_without_forwarding_the_expiry_notification() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    support::authorize(&mut sessions, &leases, handle, 300);
    common::advance(&mut leases, 120);
    sessions.advance(&leases).unwrap();
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(
        sessions.view(handle).unwrap().end_reason,
        Some(EndReason::LeaseExpired)
    );
    support::remove(&mut sessions, &leases);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Expired);
}
