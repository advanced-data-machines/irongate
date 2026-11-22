//! Cross-network/epoch identities, clock synchronization and input boundaries.
mod common;
#[path = "support/session.rs"]
mod support;

use irongate_core::{lease, session::*};

#[test]
fn wrong_network_epoch_and_reconstructed_session_owner_reject_old_work() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    let attempt = sessions.begin_consent(&leases, handle).unwrap();
    for (network, epoch) in [(lease::NetworkId(1), 2), (lease::NetworkId(2), 1)] {
        let mut other_slots = [lease::Slot::EMPTY; 4];
        let other = lease::Table::new(
            lease::PoolConfig {
                network,
                epoch,
                ..common::config()
            },
            &mut other_slots,
        )
        .unwrap();
        assert_eq!(sessions.advance(&other), Err(Error::WrongLeaseTable));
        assert_eq!(
            sessions.consent(&other, attempt, 60),
            Err(Error::WrongLeaseTable)
        );
        assert_eq!(sessions.take_effect(&other), Err(Error::WrongLeaseTable));
        assert!(!sessions.allows(&other, handle));
    }
    let mut replacement_slots = [Slot::EMPTY];
    let mut replacement = Table::new(
        Config {
            epoch: 8,
            ..support::config()
        },
        &leases,
        &mut replacement_slots,
    )
    .unwrap();
    let replacement_handle = replacement.attach(&leases, grant.handle).unwrap();
    assert_ne!(replacement_handle, handle);
    assert_eq!(replacement.logout(handle), Err(Error::StaleHandle));
    assert_eq!(
        replacement.consent(&leases, attempt, 60),
        Err(Error::StaleAttempt)
    );
    assert_eq!(sessions.view(handle).unwrap().attempt, Some(attempt));
}

#[test]
fn identical_clients_on_different_networks_have_independent_admission() {
    let mut one_lease_slots = [lease::Slot::EMPTY; 4];
    let mut two_lease_slots = [lease::Slot::EMPTY; 4];
    let mut one = lease::Table::new(common::config(), &mut one_lease_slots).unwrap();
    let network = lease::NetworkId(2);
    let mut two = lease::Table::new(
        lease::PoolConfig {
            network,
            ..common::config()
        },
        &mut two_lease_slots,
    )
    .unwrap();
    let a = common::bind(&mut one, &common::key(1), 1);
    let client = lease::ClientKey {
        network,
        ..common::key(1)
    };
    let b = common::bind(&mut two, &client, 1);
    let mut one_slots = [Slot::EMPTY];
    let mut two_slots = [Slot::EMPTY];
    let mut one_sessions = Table::new(support::config(), &one, &mut one_slots).unwrap();
    let mut two_sessions = Table::new(
        Config {
            network,
            ..support::config()
        },
        &two,
        &mut two_slots,
    )
    .unwrap();
    let a = one_sessions.attach(&one, a.handle).unwrap();
    let b = two_sessions.attach(&two, b.handle).unwrap();
    support::authorize(&mut one_sessions, &one, a, 60);
    assert!(!two_sessions.allows(&two, b));
    assert_eq!(two_sessions.logout(a), Err(Error::StaleHandle));
    support::authorize(&mut two_sessions, &two, b, 60);
    one_sessions.logout(a).unwrap();
    assert!(two_sessions.allows(&two, b));
}

#[test]
fn commands_require_observed_lease_clock_and_construction_rejects_invalid_storage() {
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(common::config(), &mut lease_slots).unwrap();
    let grant = common::bind(&mut leases, &common::key(1), 1);
    for config in [
        Config {
            epoch: 0,
            ..support::config()
        },
        Config {
            consent_seconds: 0,
            ..support::config()
        },
        Config {
            max_authorized_seconds: 0,
            ..support::config()
        },
    ] {
        assert!(matches!(
            Table::new(config, &leases, &mut [Slot::EMPTY]),
            Err(Error::InvalidConfig)
        ));
    }
    assert!(matches!(
        Table::new(support::config(), &leases, &mut []),
        Err(Error::InvalidConfig)
    ));
    assert!(matches!(
        Table::new(
            support::config(),
            &leases,
            &mut vec![Slot::EMPTY; MAX_SESSIONS + 1]
        ),
        Err(Error::InvalidConfig)
    ));
    let mut slots = [Slot::EMPTY];
    {
        let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
        let handle = sessions.attach(&leases, grant.handle).unwrap();
        common::advance(&mut leases, 1);
        assert_eq!(
            sessions.begin_consent(&leases, handle),
            Err(Error::ClockMismatch)
        );
        assert_eq!(sessions.take_effect(&leases), Err(Error::ClockMismatch));
        sessions.advance(&leases).unwrap();
        assert_eq!(sessions.now(), 1);
        sessions.begin_consent(&leases, handle).unwrap();
    }
    assert!(matches!(
        Table::new(support::config(), &leases, &mut slots),
        Err(Error::InvalidStorage)
    ));
}

#[test]
fn checked_deadlines_fail_without_consuming_the_current_attempt() {
    let config = lease::PoolConfig {
        lease_seconds: 1,
        offer_seconds: 1,
        ..common::config()
    };
    let mut lease_slots = [lease::Slot::EMPTY; 4];
    let mut leases = lease::Table::new(config, &mut lease_slots).unwrap();
    common::advance(&mut leases, u64::MAX - 5);
    let grant = common::bind(&mut leases, &common::key(1), 1);
    let mut slots = [Slot::EMPTY];
    let mut sessions = Table::new(support::config(), &leases, &mut slots).unwrap();
    let handle = sessions.attach(&leases, grant.handle).unwrap();
    assert_eq!(
        sessions.begin_consent(&leases, handle),
        Err(Error::ClockOverflow)
    );
    assert_eq!(sessions.view(handle).unwrap().phase, Phase::Leased);
    let mut short_slots = [Slot::EMPTY];
    let mut short = Table::new(
        Config {
            consent_seconds: 1,
            ..support::config()
        },
        &leases,
        &mut short_slots,
    )
    .unwrap();
    let handle = short.attach(&leases, grant.handle).unwrap();
    let attempt = short.begin_consent(&leases, handle).unwrap();
    assert_eq!(
        short.consent(&leases, attempt, 300),
        Err(Error::ClockOverflow)
    );
    assert_eq!(short.view(handle).unwrap().attempt, Some(attempt));
    assert_eq!(short.take_effect(&leases).unwrap(), None);
}
