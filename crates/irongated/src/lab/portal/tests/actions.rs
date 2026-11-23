use super::state;
use crate::lab::{
    portal::{
        bridge::{Reply, Request},
        tokens::Action,
    },
    profile,
    state::State,
};
use irongate_core::{lease, session};

fn issue(state: &mut State<'_>, id: u8) -> (Action, [u8; 32]) {
    let Reply::View(view) = state.portal_request(
        profile::CLIENT_IP.into(),
        Request::View { entropy: [id; 32] },
    ) else {
        panic!()
    };
    let (action, token, _) = view.action.unwrap();
    (action, token)
}
fn act(state: &mut State<'_>, action: Action, token: [u8; 32]) -> Reply {
    state.portal_request(profile::CLIENT_IP.into(), Request::Act { action, token })
}

#[test]
fn consent_and_logout_are_one_use_and_always_bound_to_the_tcp_peer() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    let (action, token) = issue(&mut s, 1);
    assert!(matches!(
        s.portal_request([10, 50, 10, 101].into(), Request::Act { action, token }),
        Reply::Error(403, _)
    ));
    assert!(!s.allowed());
    assert!(matches!(act(&mut s, action, token), Reply::Redirect));
    assert!(s.allowed());
    assert_eq!(s.journal.used, 1);
    assert!(matches!(act(&mut s, action, token), Reply::Error(409, _)));
    let (logout, token) = issue(&mut s, 2);
    assert_eq!(logout, Action::Logout);
    assert!(matches!(act(&mut s, logout, token), Reply::Redirect));
    assert!(!s.allowed());
    assert_eq!(s.journal.used, 2);
    assert!(matches!(act(&mut s, logout, token), Reply::Error(409, _)));
}

#[test]
fn expired_wrong_action_and_old_generation_tokens_cannot_authorize() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    let (_, token) = issue(&mut s, 1);
    assert!(matches!(
        act(&mut s, Action::Logout, token),
        Reply::Error(409, _)
    ));
    assert!(matches!(
        act(&mut s, Action::Consent, token),
        Reply::Error(409, _)
    ));
    let (_, expired) = issue(&mut s, 2);
    s.advance(60).unwrap();
    assert!(matches!(
        act(&mut s, Action::Consent, expired),
        Reply::Error(409, _)
    ));
    let (_, stale) = issue(&mut s, 3);
    let mut output = [0; 2048];
    for hex in [
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-release.hex"),
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        s.packet(
            profile::Side::Down,
            &super::common::decode(hex),
            &mut output,
        );
    }
    assert!(matches!(
        act(&mut s, Action::Consent, stale),
        Reply::Error(409, _)
    ));
    assert!(!s.allowed());
    let (action, fresh) = issue(&mut s, 4);
    assert!(matches!(act(&mut s, action, fresh), Reply::Redirect));
    assert!(s.allowed());
}

#[test]
fn token_capacity_and_readonly_status_have_explicit_behavior() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    let first = issue(&mut s, 1);
    for id in 2..=8 {
        assert_eq!(issue(&mut s, id), first);
    }
    let Reply::View(view) = s.portal_request(profile::CLIENT_IP.into(), Request::Status) else {
        panic!()
    };
    assert!(view.action.is_none());
    assert!(!view.allowed);
    s.advance(60).unwrap();
    issue(&mut s, 5);
}

#[test]
fn full_journal_denies_new_admission_but_preserves_the_reserved_stop() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    let (action, token) = issue(&mut s, 1);
    act(&mut s, action, token);
    s.control(b"record-pressure");
    assert_eq!(s.journal.used, 15);
    let (action, token) = issue(&mut s, 2);
    assert!(matches!(act(&mut s, action, token), Reply::Redirect));
    assert!(!s.allowed());
    assert_eq!(s.journal.used, 16);
    let (action, token) = issue(&mut s, 3);
    assert!(matches!(act(&mut s, action, token), Reply::Error(503, _)));
    assert!(!s.allowed());
    assert_eq!(s.journal.used, 16);
}
