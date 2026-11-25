use super::*;
#[test]
fn external_accept_requires_current_attempt_and_exact_policy_ack() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(lease_config(), &mut lease_slots).unwrap();
    let grant = bind(&mut leases);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(config(), &leases, &mut slots).unwrap();
    let h = sessions.attach(&leases, grant.handle).unwrap();
    let old = sessions.begin_radius(&leases, h).unwrap();
    let current = sessions.begin_radius(&leases, h).unwrap();
    assert_eq!(
        sessions.consent(&leases, current, 5),
        Err(Error::StaleAttempt)
    );
    assert_eq!(
        sessions.radius_accept(&leases, old, 5),
        Err(Error::StaleAttempt)
    );
    assert!(!sessions.allows(&leases, h));
    sessions.radius_accept(&leases, current, 5).unwrap();
    assert!(!sessions.allows(&leases, h));
    let effect = sessions.take_effect(&leases).unwrap().unwrap();
    sessions.complete(&leases, effect.token, true).unwrap();
    assert!(sessions.allows(&leases, h));
    assert_eq!(sessions.view(h).unwrap().provider, Some(Provider::Radius));
    leases.advance(5, &mut [None; 4]).unwrap();
    sessions.advance(&leases).unwrap();
    assert!(!sessions.allows(&leases, h));
}
#[test]
fn cancelled_expired_and_reused_sessions_reject_late_external_results() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(lease_config(), &mut lease_slots).unwrap();
    let grant = bind(&mut leases);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(config(), &leases, &mut slots).unwrap();
    let h = sessions.attach(&leases, grant.handle).unwrap();
    let attempt = sessions.begin_radius(&leases, h).unwrap();
    sessions.logout(h).unwrap();
    assert_eq!(
        sessions.radius_accept(&leases, attempt, 5),
        Err(Error::StaleAttempt)
    );
    sessions.forget(h).unwrap();
    let new = sessions.attach(&leases, grant.handle).unwrap();
    assert_ne!(new.generation(), h.generation());
    assert_eq!(
        sessions.radius_accept(&leases, attempt, 5),
        Err(Error::StaleAttempt)
    );
    let current = sessions.begin_radius(&leases, new).unwrap();
    leases.advance(10, &mut [None; 4]).unwrap();
    sessions.advance(&leases).unwrap();
    assert_eq!(
        sessions.radius_accept(&leases, current, 5),
        Err(Error::StaleAttempt)
    );
    assert_eq!(
        sessions.view(new).unwrap().end_reason,
        Some(EndReason::AuthenticationExpired)
    );
    let current = sessions.begin_radius(&leases, new).unwrap();
    sessions
        .radius_deny(&leases, current, EndReason::AuthenticationRejected)
        .unwrap();
    assert!(!sessions.allows(&leases, new));
    assert!(sessions.take_effect(&leases).unwrap().is_none());
}
