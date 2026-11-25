//! Exercise the real admission/removal boundary, without any backend socket.
use super::state;
use crate::lab::{
    accounting::{
        Outbox, UserName,
        model::{Kind, Outcome, Receipt},
    },
    portal::bridge::Reply,
    radius::{Decision, Outcome as Auth},
};
use irongate_core::{lease, session};
#[test]
fn accepted_identity_reserves_before_apply_and_logout_revokes_during_start_failure() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    s.provider = session::Provider::Radius;
    s.accounting = Outbox::enabled([1; 16], 2);
    let h = s.active.unwrap();
    let attempt = s.sessions.begin_radius(&s.leases, h).unwrap();
    let decision = Decision {
        outcome: Auth::Accept { seconds: 5 },
        elapsed_ms: 1,
        invalid: 0,
        sends: 1,
    };
    assert!(matches!(
        s.radius_result(attempt, decision, None),
        Reply::Error(503, _)
    ));
    assert!(!s.allowed());
    assert_eq!(s.accounting.snapshot(0).retained, 0);
    let attempt = s.sessions.begin_radius(&s.leases, h).unwrap();
    assert!(matches!(
        s.radius_result(attempt, decision, UserName::new(b"lab-user")),
        Reply::Redirect
    ));
    assert!(s.allowed());
    let start = s.accounting.next(0).unwrap();
    assert_eq!(start.record.kind, Kind::Start);
    s.accounting.complete(
        Receipt {
            token: start.token,
            outcome: Outcome::Timeout,
            invalid: 0,
        },
        1,
    );
    assert!(!s.accounting.can_admit());
    assert!(s.allowed());
    assert_eq!(s.control(b"logout")["allowed"], false);
    assert_eq!(s.accounting.snapshot(1).pending, 2);
    let retry = s.accounting.next(3).unwrap();
    assert_eq!(retry.record.kind, Kind::Start);
    s.accounting.complete(
        Receipt {
            token: retry.token,
            outcome: Outcome::Ack,
            invalid: 0,
        },
        3,
    );
    let stop = s.accounting.next(3).unwrap();
    assert_eq!(stop.record.kind, Kind::Stop);
    assert_eq!(stop.record.cause, 1);
    s.accounting.complete(
        Receipt {
            token: stop.token,
            outcome: Outcome::Ack,
            invalid: 0,
        },
        3,
    );
    assert!(s.accounting.can_admit());
}
#[test]
fn authorization_expiry_freezes_accounting_and_fixture_consent_does_not_export() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    s.provider = session::Provider::Radius;
    s.accounting = Outbox::enabled([1; 16], 2);
    let h = s.active.unwrap();
    let attempt = s.sessions.begin_radius(&s.leases, h).unwrap();
    let decision = Decision {
        outcome: Auth::Accept { seconds: 5 },
        elapsed_ms: 1,
        invalid: 0,
        sends: 1,
    };
    assert!(matches!(
        s.radius_result(attempt, decision, UserName::new(b"lab-user")),
        Reply::Redirect
    ));
    let start = s.accounting.next(0).unwrap();
    s.accounting.complete(
        Receipt {
            token: start.token,
            outcome: Outcome::Ack,
            invalid: 0,
        },
        0,
    );
    s.advance(5).unwrap();
    assert!(!s.allowed());
    let stop = s.accounting.next(5).unwrap();
    assert_eq!(stop.record.cause, 5);
    assert_eq!(stop.record.session_seconds, 5);
    s.accounting.complete(
        Receipt {
            token: stop.token,
            outcome: Outcome::Ack,
            invalid: 0,
        },
        5,
    );
    assert_eq!(s.control(b"consent")["allowed"], true);
    assert!(s.accounting.next(5).is_none());
    s.control(b"logout");
    assert_eq!(s.accounting.snapshot(5).retained, 0);
}

#[test]
fn accounting_mailbox_rejects_stale_receipts_and_reports_worker_disconnection() {
    use crate::lab::portal::bridge::Bridge;
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut s = state(&mut leases, &mut sessions);
    s.accounting = Outbox::enabled([1; 16], 2);
    let h = s.active.unwrap();
    let l = s.sessions.view(h).unwrap().lease;
    assert!(
        s.accounting
            .reserve(h, l, UserName::new(b"lab-user").unwrap(), 0)
    );
    s.accounting.activate(h, l, 0);
    let (bridge, worker) = Bridge::pair().unwrap();
    bridge.budget(&mut s);
    let start = worker.accounting.try_recv().unwrap();
    assert!(s.accounting.snapshot(0).in_flight);
    let mut stale = start.token;
    stale.sequence += 1;
    worker
        .accounting_acks
        .try_send(Receipt {
            token: stale,
            outcome: Outcome::Ack,
            invalid: 0,
        })
        .unwrap();
    bridge.budget(&mut s);
    assert_eq!(s.accounting.snapshot(0).stale, 1);
    assert_eq!(s.accounting.snapshot(0).acknowledged, 0);
    worker
        .accounting_acks
        .try_send(Receipt {
            token: start.token,
            outcome: Outcome::Ack,
            invalid: 0,
        })
        .unwrap();
    bridge.budget(&mut s);
    assert_eq!(s.accounting.snapshot(0).acknowledged, 1);
    s.accounting.stop(h, l, 0, 1);
    drop(worker);
    bridge.budget(&mut s);
    assert!(s.accounting.snapshot(0).unavailable);
    assert_eq!(s.accounting.snapshot(0).retained, 1);
}
