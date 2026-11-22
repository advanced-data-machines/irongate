//! Offline DORA, renewal, release and conflict quarantine. Opens no sockets.

#[path = "../tests/common/mod.rs"]
mod common;
#[path = "support/lease.rs"]
mod lease;

use irongate_core::lease::{EventKind, Outcome, Phase, Slot, Table};

fn main() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(lease::config(), &mut slots).unwrap();
    let mut output = [0; 2048];
    let mut events = [None; 4];
    for (now, name, fixture) in [
        (0, "DISCOVER → OFFER", common::DISCOVER),
        (
            1,
            "REQUEST → ACK",
            include_str!("../tests/fixtures/selecting.hex"),
        ),
        (
            60,
            "renew → ACK",
            include_str!("../tests/fixtures/renew.hex"),
        ),
        (
            61,
            "RELEASE → silent",
            include_str!("../tests/fixtures/release.hex"),
        ),
        (
            62,
            "DISCOVER → OFFER",
            include_str!("../tests/fixtures/discover-requested.hex"),
        ),
        (
            63,
            "DECLINE → quarantine",
            include_str!("../tests/fixtures/decline.hex"),
        ),
    ] {
        let count = table.advance(now, &mut events).unwrap();
        assert_eq!(count, 0);
        let input = common::decode(fixture);
        let (outcome, length) = lease::exchange(&mut table, &input, &mut output);
        match outcome {
            Outcome::Offer(grant) | Outcome::Ack(grant) => {
                let view = table.view(grant.handle).unwrap();
                println!(
                    "t={now:>2}: {name}; {:?}, address={:?}, expires={}, reply={length} bytes",
                    view.phase, grant.address, view.expires_at
                );
            }
            Outcome::Released(handle) => assert!(table.view(handle).is_none()),
            Outcome::Declined(handle) => {
                assert_eq!(table.view(handle).unwrap().phase, Phase::Declined)
            }
            _ => panic!("unexpected fixture outcome"),
        }
        if length == 0 {
            println!("t={now:>2}: {name}; no reply");
        }
    }
    let count = table.advance(93, &mut events).unwrap();
    assert_eq!(count, 1);
    assert_eq!(events[0].unwrap().kind, EventKind::QuarantineEnded);
    println!("t=93: conflict quarantine ended; address available again");
    println!("Offline only: no sockets, host network changes, or upstream authorization.");
}
