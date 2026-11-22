//! Complete fixture lifecycles with identical setup outside measured replays.

#[path = "../tests/common/mod.rs"]
mod common;
#[path = "support/lease.rs"]
mod lease;

use irongate_core::lease::{Outcome, Slot, Table};
use std::hint::black_box;

fn main() {
    let iterations: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "100000".to_owned())
        .parse()
        .expect("iteration count must fit u32");
    let fixtures = [
        include_str!("../tests/fixtures/discover-requested.hex"),
        include_str!("../tests/fixtures/selecting.hex"),
        include_str!("../tests/fixtures/renew.hex"),
        include_str!("../tests/fixtures/release.hex"),
        include_str!("../tests/fixtures/discover-requested.hex"),
        include_str!("../tests/fixtures/decline.hex"),
    ]
    .map(common::decode);
    let mut slots = vec![Slot::EMPTY; 4]; // One startup allocation, no growth.
    let mut table = Table::new(lease::config(), &mut slots).unwrap();
    let mut output = [0; 2048];
    let mut events = [None; 4];
    // Always perform one warmup, including in the zero-iteration baseline.
    for cycle in 0..=iterations {
        let now = u64::from(cycle) * 31;
        assert_eq!(table.advance(now, &mut events).unwrap(), 0);
        for fixture in &fixtures {
            let (outcome, length) = lease::exchange(
                black_box(&mut table),
                black_box(fixture),
                black_box(&mut output),
            );
            match outcome {
                Outcome::Offer(_)
                | Outcome::Ack(_)
                | Outcome::Released(_)
                | Outcome::Declined(_) => (),
                _ => panic!("unexpected fixture outcome"),
            }
            black_box(&output[..length]);
        }
        // Consume expiry effects before the next cycle reuses this address.
        assert_eq!(table.advance(now + 30, &mut events).unwrap(), 1);
        black_box(events[0].take().unwrap());
    }
}
