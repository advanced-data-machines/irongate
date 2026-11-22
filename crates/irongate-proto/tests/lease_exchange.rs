//! Pure lease ownership + wire encoding, with no OS/network adapter.

mod common;
#[path = "../examples/support/lease.rs"]
mod lease;

use irongate_core::lease::{Outcome, Phase, Slot, Table};

#[test]
fn complete_exchange_matches_independent_offer_and_ack_for_both_identities() {
    for (discover, selecting, offer, ack) in [
        (
            common::DISCOVER,
            include_str!("fixtures/selecting.hex"),
            include_str!("fixtures/offer.hex"),
            include_str!("fixtures/ack.hex"),
        ),
        (
            common::TAGGED,
            include_str!("fixtures/selecting-vlan-client-id.hex"),
            include_str!("fixtures/offer-vlan-client-id.hex"),
            include_str!("fixtures/ack-vlan-client-id.hex"),
        ),
    ] {
        let mut slots = [Slot::EMPTY; 4];
        let mut table = Table::new(lease::config(), &mut slots).unwrap();
        let mut output = [0; 2048];
        for (input, expected, phase) in [
            (discover, offer, Phase::Offered),
            (selecting, ack, Phase::Bound),
        ] {
            let (outcome, length) =
                lease::exchange(&mut table, &common::decode(input), &mut output);
            let grant = match outcome {
                Outcome::Offer(grant) | Outcome::Ack(grant) => grant,
                _ => panic!("unexpected outcome"),
            };
            assert_eq!(table.view(grant.handle).unwrap().phase, phase);
            assert_eq!(&output[..length], common::decode(expected));
        }
    }
}

#[test]
fn request_without_matching_offer_encodes_independently_expected_nak() {
    let mut slots = [Slot::EMPTY; 4];
    let mut table = Table::new(lease::config(), &mut slots).unwrap();
    let mut output = [0; 2048];
    let (outcome, length) = lease::exchange(
        &mut table,
        &common::decode(include_str!("fixtures/selecting.hex")),
        &mut output,
    );
    assert!(matches!(outcome, Outcome::Nak(_)));
    assert_eq!(
        &output[..length],
        common::decode(include_str!("fixtures/nak.hex"))
    );
}
