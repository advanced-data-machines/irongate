//! Internal overflow boundaries that cannot be reached by a practical replay.

use super::*;
use crate::lease;

fn config() -> Config {
    Config {
        network: lease::NetworkId(1),
        epoch: 7,
        consent_seconds: 10,
        max_authorized_seconds: 300,
    }
}

#[test]
fn exhausted_slot_generation_retires_without_aliasing_old_handles() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(lease_config(), &mut lease_slots).unwrap();
    let grant = bind(&mut leases);
    let mut slots = [Slot::EMPTY; 2];
    slots[0].generation = u64::MAX;
    let mut table = Table::new(config(), &leases, &mut slots).unwrap();
    let handle = table.attach(&leases, grant.handle).unwrap();
    assert_eq!(handle.index, 1);
    assert_eq!(handle.generation(), 1);
    assert!(table.slots[0].retired);
}

#[test]
fn exhausted_sequences_leave_attempt_and_pending_effect_unchanged() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(lease_config(), &mut lease_slots).unwrap();
    let grant = bind(&mut leases);
    let mut slots = [Slot::EMPTY];
    let mut table = Table::new(config(), &leases, &mut slots).unwrap();
    let handle = table.attach(&leases, grant.handle).unwrap();
    table.sequence = u64::MAX - 1;
    let attempt = table.begin_consent(&leases, handle).unwrap();
    assert_eq!(
        table.begin_consent(&leases, handle),
        Err(Error::SequenceExhausted)
    );
    assert_eq!(table.view(handle).unwrap().attempt, Some(attempt));
    table.consent(&leases, attempt, 60).unwrap();
    assert_eq!(table.take_effect(&leases), Err(Error::SequenceExhausted));
    assert_eq!(
        table.view(handle).unwrap().pending,
        Some(EffectKind::Apply { until: 60 })
    );
    assert_eq!(table.view(handle).unwrap().in_flight, None);
    assert!(!table.allows(&leases, handle));
}

fn lease_config() -> lease::PoolConfig {
    lease::PoolConfig {
        network: lease::NetworkId(1),
        epoch: 1,
        server: [10, 0, 0, 1],
        prefix: 24,
        first: [10, 0, 0, 20],
        last: [10, 0, 0, 23],
        lease_seconds: 120,
        offer_seconds: 10,
        decline_seconds: 30,
    }
}

fn bind(leases: &mut lease::Table<'_>) -> lease::Grant {
    let client = lease::ClientKey {
        network: lease::NetworkId(1),
        identity: lease::ClientIdentity::ethernet([2, 0, 0, 0, 0, 1]).unwrap(),
    };
    let lease::Outcome::Offer(grant) = leases
        .apply(lease::Command::Discover {
            client: &client,
            xid: 1,
            requested: None,
        })
        .unwrap()
    else {
        panic!()
    };
    leases
        .apply(lease::Command::Request {
            client: &client,
            xid: 1,
            mode: lease::RequestMode::Selecting {
                server: lease_config().server,
                address: grant.address,
            },
        })
        .unwrap();
    grant
}
