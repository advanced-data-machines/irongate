//! Repeated admission/cleanup with fixed storage and simulated worker results.

#[path = "../tests/common/mod.rs"]
mod common;
#[path = "../tests/support/session.rs"]
mod support;

use irongate_core::{lease, session};
use std::hint::black_box;

fn main() {
    let iterations: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "100000".to_owned())
        .parse()
        .expect("iteration count must fit u32");
    let mut lease_slots = vec![lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let mut slots = vec![session::Slot::EMPTY; 2];
    let mut sessions = session::Table::new(support::config(), &leases, &mut slots).unwrap();
    let client = common::key(1);
    // Always run one warmup; baseline/replay have equal argument lengths/setup.
    for cycle in 0..=iterations {
        support::tick(&mut leases, &mut sessions, u64::from(cycle) * 10);
        let grant = common::bind(black_box(&mut leases), black_box(&client), cycle);
        let handle = sessions.attach(&leases, grant.handle).unwrap();
        assert!(!black_box(sessions.allows(&leases, handle)));
        let stale = sessions.begin_consent(&leases, handle).unwrap();
        let attempt = sessions.begin_consent(&leases, handle).unwrap();
        assert_eq!(
            sessions.consent(&leases, stale, 5),
            Err(session::Error::StaleAttempt)
        );
        sessions.consent(&leases, black_box(attempt), 5).unwrap();
        let apply = sessions.take_effect(&leases).unwrap().unwrap();
        sessions
            .complete(&leases, black_box(apply.token), true)
            .unwrap();
        assert!(black_box(sessions.allows(&leases, handle)));
        if cycle % 2 == 0 {
            sessions.logout(handle).unwrap();
        } else {
            support::tick(&mut leases, &mut sessions, u64::from(cycle) * 10 + 5);
        }
        assert!(!black_box(sessions.allows(&leases, handle)));
        let removal = sessions.take_effect(&leases).unwrap().unwrap();
        sessions.complete(&leases, removal.token, false).unwrap();
        let retry = sessions.take_effect(&leases).unwrap().unwrap();
        assert_ne!(retry.token, removal.token);
        assert_eq!(
            sessions.complete(&leases, removal.token, true),
            Err(session::Error::StaleEffect)
        );
        sessions
            .complete(&leases, black_box(retry.token), true)
            .unwrap();
        black_box(sessions.view(handle).unwrap());
        sessions.forget(handle).unwrap();
        assert!(sessions.view(handle).is_none());
        let released = leases
            .apply(lease::Command::Release {
                client: &client,
                address: grant.address,
                server: common::config().server,
            })
            .unwrap();
        assert!(matches!(released, lease::Outcome::Released(_)));
        assert_eq!(sessions.lease_lost(grant.handle), 0);
    }
}
