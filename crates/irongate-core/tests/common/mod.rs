//! Allocating fixture storage is setup only; the core borrows it.
#![allow(dead_code)]

use irongate_core::lease::*;

pub fn config() -> PoolConfig {
    PoolConfig {
        network: NetworkId(1),
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
pub fn key(id: u8) -> ClientKey {
    ClientKey {
        network: NetworkId(1),
        identity: ClientIdentity::ethernet([2, 0, 0, 0, 0, id]).unwrap(),
    }
}
pub fn discover(table: &mut Table<'_>, client: &ClientKey, xid: u32) -> Grant {
    match table
        .apply(Command::Discover {
            client,
            xid,
            requested: None,
        })
        .unwrap()
    {
        Outcome::Offer(grant) => grant,
        other => panic!("Expected OFFER: {other:?}"),
    }
}
pub fn select(table: &mut Table<'_>, client: &ClientKey, xid: u32, address: [u8; 4]) -> Outcome {
    table
        .apply(Command::Request {
            client,
            xid,
            mode: RequestMode::Selecting {
                server: config().server,
                address,
            },
        })
        .unwrap()
}
pub fn bind(table: &mut Table<'_>, client: &ClientKey, xid: u32) -> Grant {
    let offer = discover(table, client, xid);
    match select(table, client, xid, offer.address) {
        Outcome::Ack(grant) => grant,
        other => panic!("Expected ACK: {other:?}"),
    }
}
pub fn advance(table: &mut Table<'_>, now: u64) -> Vec<Event> {
    let mut events = [None; MAX_LEASES];
    let count = table.advance(now, &mut events).unwrap();
    events[..count].iter().map(|event| event.unwrap()).collect()
}
