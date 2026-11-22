//! Exercise otherwise unreachable generation exhaustion without exposing setters.
use super::model::State;
use super::*;

#[test]
fn exhausted_generation_retires_instead_of_revalidating_an_old_handle() {
    let config = PoolConfig {
        network: NetworkId(1),
        epoch: 1,
        server: [10, 0, 0, 1],
        prefix: 24,
        first: [10, 0, 0, 20],
        last: [10, 0, 0, 21],
        lease_seconds: 120,
        offer_seconds: 10,
        decline_seconds: 30,
    };
    let mut slots = [
        Slot {
            generation: u64::MAX,
            ..Slot::EMPTY
        },
        Slot::EMPTY,
    ];
    let mut table = Table::new(config, &mut slots).unwrap();
    let client = ClientKey {
        network: NetworkId(1),
        identity: ClientIdentity::ethernet([2, 0, 0, 0, 0, 1]).unwrap(),
    };
    let outcome = table
        .apply(Command::Discover {
            client: &client,
            xid: 1,
            requested: Some(config.first),
        })
        .unwrap();
    assert!(
        matches!(outcome, Outcome::Offer(grant) if grant.address == config.last && grant.handle.generation() == 1)
    );
    assert_eq!(table.slots[0].state, State::Retired);
}
