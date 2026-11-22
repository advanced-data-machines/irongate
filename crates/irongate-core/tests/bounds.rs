//! Capacity, clocks, ingress identity and generation handling.
mod common;
use common::*;
use irongate_core::lease::*;

#[test]
fn full_pool_preserves_existing_bindings_and_reuse_rejects_stale_handles() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let clients = [key(1), key(2), key(3), key(4)];
    let grants = clients.map(|client| bind(&mut table, &client, 1));
    assert_eq!(
        table.apply(Command::Discover {
            client: &key(5),
            xid: 2,
            requested: None
        }),
        Err(Error::Exhausted)
    );
    for grant in grants {
        assert_eq!(table.view(grant.handle).unwrap().phase, Phase::Bound);
    }
    table
        .apply(Command::Release {
            client: &clients[0],
            address: grants[0].address,
            server: config().server,
        })
        .unwrap();
    let replacement = discover(&mut table, &key(5), 3);
    assert_eq!(replacement.address, grants[0].address);
    assert_ne!(
        replacement.handle.generation(),
        grants[0].handle.generation()
    );
    assert!(table.view(grants[0].handle).is_none());
}

#[test]
fn expiration_events_cannot_be_lost_to_a_short_output_buffer() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let grant = bind(&mut table, &key(1), 1);
    let offer = discover(&mut table, &key(2), 1);
    assert_eq!(
        table.advance(120, &mut [None]),
        Err(Error::EventCapacity { required: 2 })
    );
    assert_eq!(table.now(), 0);
    assert_eq!(table.view(grant.handle).unwrap().phase, Phase::Bound);
    assert_eq!(table.view(offer.handle).unwrap().phase, Phase::Offered);
    assert_eq!(advance(&mut table, 120).len(), 2);
    assert!(table.view(grant.handle).is_none());
}

#[test]
fn clocks_cannot_regress_or_wrap_deadlines() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    advance(&mut table, 20);
    assert_eq!(table.advance(19, &mut []), Err(Error::ClockRegressed));
    assert_eq!(table.now(), 20);
    advance(&mut table, u64::MAX);
    assert_eq!(
        table.apply(Command::Discover {
            client: &key(1),
            xid: 1,
            requested: None
        }),
        Err(Error::ClockOverflow)
    );
}

#[test]
fn network_and_arena_epochs_prevent_cross_context_handles() {
    let client = key(1);
    let mut one = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut one).unwrap();
    let grant = bind(&mut table, &client, 1);
    for (network, epoch) in [(NetworkId(1), 2), (NetworkId(2), 1)] {
        let mut slots = [Slot::EMPTY; 4];
        let mut other = Table::new(
            PoolConfig {
                network,
                epoch,
                ..config()
            },
            &mut slots,
        )
        .unwrap();
        let own_client = ClientKey { network, ..client };
        bind(&mut other, &own_client, 1);
        assert!(other.view(grant.handle).is_none());
    }
    let wrong = ClientKey {
        network: NetworkId(2),
        ..client
    };
    assert_eq!(
        table.apply(Command::Discover {
            client: &wrong,
            xid: 2,
            requested: None
        }),
        Err(Error::WrongNetwork)
    );
}

#[test]
fn client_identifiers_are_bounded_and_separate_from_hardware_identity() {
    assert_eq!(ClientIdentity::option_61(&[0]), Err(Error::InvalidClient));
    assert_eq!(
        ClientIdentity::option_61(&[0; 256]),
        Err(Error::InvalidClient)
    );
    assert_eq!(ClientIdentity::ethernet([0; 6]), Err(Error::InvalidClient));
    assert_eq!(ClientIdentity::ethernet([1; 6]), Err(Error::InvalidClient));
    assert!(ClientIdentity::option_61(&[0; 255]).is_ok());
    let hardware = ClientIdentity::ethernet([2, 0, 0, 0, 0, 1]).unwrap();
    let opaque = ClientIdentity::option_61(&[2, 0, 0, 0, 0, 1]).unwrap();
    assert_ne!(hardware, opaque);
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(config(), &mut slots).unwrap();
    let a = discover(
        &mut table,
        &ClientKey {
            network: NetworkId(1),
            identity: hardware,
        },
        1,
    );
    let b = discover(
        &mut table,
        &ClientKey {
            network: NetworkId(1),
            identity: opaque,
        },
        1,
    );
    assert_ne!(a.address, b.address);
}

#[test]
fn invalid_pool_and_nonempty_or_mismatched_storage_are_rejected() {
    for bad in [
        PoolConfig {
            epoch: 0,
            ..config()
        },
        PoolConfig {
            prefix: 31,
            ..config()
        },
        PoolConfig {
            lease_seconds: 0,
            ..config()
        },
        PoolConfig {
            first: [10, 0, 0, 1],
            ..config()
        },
        PoolConfig {
            last: [10, 0, 0, 255],
            ..config()
        },
        PoolConfig {
            last: [10, 0, 0, 19],
            ..config()
        },
        PoolConfig {
            prefix: 16,
            last: [10, 0, 2, 0],
            ..config()
        },
    ] {
        assert!(matches!(
            Table::new(bad, &mut [Slot::EMPTY; 4]),
            Err(Error::InvalidConfig)
        ));
    }
    assert!(matches!(
        Table::new(config(), &mut [Slot::EMPTY; 3]),
        Err(Error::InvalidStorage)
    ));
    let mut slots = [Slot::EMPTY; 4];
    {
        let mut table = Table::new(config(), &mut slots).unwrap();
        discover(&mut table, &key(1), 1);
    }
    assert!(matches!(
        Table::new(config(), &mut slots),
        Err(Error::InvalidStorage)
    ));
}
