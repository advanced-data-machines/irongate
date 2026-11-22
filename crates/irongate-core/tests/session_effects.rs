//! In-flight operations must quiesce before cleanup or address reuse.
mod common;
#[path = "support/session.rs"]
mod support;

use irongate_core::{lease, session::*};

#[test]
fn logout_cancels_an_unsent_grant_without_external_work() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let attempt = sessions.begin_consent(&leases, handle).unwrap();
    sessions.consent(&leases, attempt, 60).unwrap();
    sessions.logout(handle).unwrap();
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoked);
    sessions.forget(handle).unwrap();
    assert_eq!(
        sessions.consent(&leases, attempt, 60),
        Err(Error::StaleAttempt)
    );
}

#[test]
fn late_successful_apply_after_logout_can_only_lead_to_removal() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let apply = support::apply(&mut sessions, &leases, handle, 60);
    sessions.logout(handle).unwrap();
    assert_eq!(sessions.take_effect(&leases).unwrap(), None); // No Remove can race the Apply.
    assert_eq!(sessions.forget(handle), Err(Error::InvalidState));
    sessions.complete(&leases, apply.token, true).unwrap();
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoking);
    let removal = sessions.take_effect(&leases).unwrap().unwrap();
    assert_eq!(
        removal.kind,
        EffectKind::Remove {
            reason: EndReason::Logout
        }
    );
    assert_eq!(
        sessions.complete(&leases, apply.token, true),
        Err(Error::StaleEffect)
    );
    sessions.complete(&leases, removal.token, true).unwrap();
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoked);
}

#[test]
fn partial_apply_failure_requires_cleanup_and_failed_remove_is_retained_with_new_token() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let apply = support::apply(&mut sessions, &leases, handle, 60);
    sessions.complete(&leases, apply.token, false).unwrap();
    assert!(!sessions.allows(&leases, handle));
    let removal = sessions.take_effect(&leases).unwrap().unwrap();
    assert_eq!(
        removal.kind,
        EffectKind::Remove {
            reason: EndReason::ApplyFailed
        }
    );
    sessions.complete(&leases, removal.token, false).unwrap();
    let view = sessions.view(handle).unwrap();
    assert_eq!(view.phase, Phase::Revoking);
    assert_eq!(view.removal_failures, 1);
    assert_eq!(sessions.forget(handle), Err(Error::InvalidState));
    let retry = sessions.take_effect(&leases).unwrap().unwrap();
    assert_ne!(retry.token, removal.token);
    assert_eq!(retry.kind, removal.kind);
    assert_eq!(
        sessions.complete(&leases, removal.token, true),
        Err(Error::StaleEffect)
    );
    sessions.complete(&leases, retry.token, true).unwrap();
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Revoked);
    assert_eq!(
        sessions.view(handle).unwrap().end_reason,
        Some(EndReason::ApplyFailed)
    );
}

#[test]
fn expired_in_flight_grant_is_removed_and_never_authorized() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let apply = support::apply(&mut sessions, &leases, handle, 5);
    support::tick(&mut leases, &mut sessions, 5);
    assert_eq!(sessions.take_effect(&leases).unwrap(), None);
    sessions.complete(&leases, apply.token, true).unwrap();
    assert!(!sessions.allows(&leases, handle));
    assert_eq!(
        support::remove(&mut sessions, &leases).kind,
        EffectKind::Remove {
            reason: EndReason::AuthorizationExpired
        }
    );
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Expired);
}

#[test]
fn failed_removal_on_one_session_does_not_starve_another_pending_operation() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grants = [
        common::bind(&mut leases, &common::key(1), 1),
        common::bind(&mut leases, &common::key(2), 1),
    ];
    let mut slots = [Slot::EMPTY; 2];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let one = sessions.attach(&leases, grants[0].handle).unwrap();
    let two = sessions.attach(&leases, grants[1].handle).unwrap();
    support::authorize(&mut sessions, &leases, one, 60);
    sessions.logout(one).unwrap();
    let removal = sessions.take_effect(&leases).unwrap().unwrap();
    sessions.complete(&leases, removal.token, false).unwrap();
    let attempt = sessions.begin_consent(&leases, two).unwrap();
    sessions.consent(&leases, attempt, 60).unwrap();
    let next = sessions.take_effect(&leases).unwrap().unwrap();
    assert_eq!(next.token.session(), two);
    sessions.complete(&leases, next.token, true).unwrap();
    assert!(sessions.allows(&leases, two));
    assert_eq!(
        sessions
            .take_effect(&leases)
            .unwrap()
            .unwrap()
            .token
            .session(),
        one
    );
}
