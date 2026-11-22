//! Exact pilot capacity and conflict quarantine under address pressure.
mod common;
use common::*;
use irongate_core::lease::*;

#[test]
fn maximum_pool_has_one_distinct_address_per_slot_and_never_evicts_on_exhaustion() {
    let settings = PoolConfig {
        prefix: 16,
        first: [10, 0, 0, 2],
        last: [10, 0, 1, 1],
        ..config()
    };
    let mut slots = vec![Slot::EMPTY; MAX_LEASES];
    let mut table = Table::new(settings, &mut slots).unwrap();
    let mut handles = Vec::new();
    for id in 0..MAX_LEASES {
        let grant = discover(&mut table, &key(id as u8), id as u32);
        assert_eq!(
            u32::from_be_bytes(grant.address),
            u32::from_be_bytes(settings.first) + id as u32
        );
        handles.push(grant.handle);
    }
    let extra = ClientKey {
        network: settings.network,
        identity: ClientIdentity::option_61(&[0, 1]).unwrap(),
    };
    assert_eq!(
        table.apply(Command::Discover {
            client: &extra,
            xid: 1,
            requested: None
        }),
        Err(Error::Exhausted)
    );
    assert!(handles.iter().all(|handle| table.view(*handle).is_some()));
    // Every expiry effect must fit, including the exact 256-slot boundary.
    let mut events = vec![None; MAX_LEASES];
    assert_eq!(
        table.advance(10, &mut events[..MAX_LEASES - 1]),
        Err(Error::EventCapacity {
            required: MAX_LEASES
        })
    );
    assert_eq!(table.now(), 0);
    assert_eq!(table.advance(10, &mut events).unwrap(), MAX_LEASES);
    let replacement = discover(&mut table, &extra, 2);
    assert_eq!(replacement.address, settings.first);
    assert!(handles.iter().all(|handle| table.view(*handle).is_none()));
    let too_many = PoolConfig {
        last: [10, 0, 1, 2],
        ..settings
    };
    assert!(matches!(
        Table::new(too_many, &mut slots),
        Err(Error::InvalidConfig)
    ));
}

#[test]
fn sole_address_cannot_be_reused_until_conflict_hold_ends() {
    let mut slots = [Slot::EMPTY];
    let settings = PoolConfig {
        last: config().first,
        ..config()
    };
    let mut table = Table::new(settings, &mut slots).unwrap();
    let owner = key(1);
    let other = key(2);
    let offered = discover(&mut table, &owner, 1);
    table
        .apply(Command::Decline {
            client: &owner,
            address: offered.address,
            server: settings.server,
        })
        .unwrap();
    advance(&mut table, 29);
    assert_eq!(
        table.apply(Command::Discover {
            client: &other,
            xid: 2,
            requested: Some(offered.address)
        }),
        Err(Error::Exhausted)
    );
    assert_eq!(advance(&mut table, 30)[0].kind, EventKind::QuarantineEnded);
    let next = discover(&mut table, &other, 2);
    assert_eq!(next.address, offered.address);
    assert_ne!(next.handle, offered.handle);
    assert!(table.view(offered.handle).is_none());
}
