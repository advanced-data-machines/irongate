//! Independent wire fixtures and fail-before-write routing/ARP boundaries.
mod common;
use common::decode;
use irongate_proto::{Error, arp, routed};

fn endpoint() -> arp::Endpoint {
    arp::Endpoint {
        mac: [2, 0, 0, 0, 0, 1],
        ip: [10, 50, 10, 1],
        peer_mac: [2, 0, 0, 0, 0, 2],
        peer_ip: [10, 50, 10, 100],
    }
}
fn route() -> routed::Route {
    routed::Route {
        source_mac: endpoint().peer_mac,
        gateway_mac: endpoint().mac,
        source_ip: endpoint().peer_ip,
        destination_ip: [192, 0, 2, 80],
        output_mac: [2, 0, 0, 0, 1, 1],
        next_hop_mac: [2, 0, 0, 0, 1, 2],
    }
}

#[test]
fn arp_reply_matches_independent_fixture_and_ignores_other_identities() {
    let input = decode(include_str!("fixtures/arp-request.hex"));
    let mut output = [0xaa; 64];
    assert_eq!(arp::reply(&input, endpoint(), &mut output), Ok(Some(42)));
    assert_eq!(
        &output[..42],
        decode(include_str!("fixtures/arp-reply.hex"))
    );
    assert_eq!(&output[42..], &[0xaa; 22]);
    for index in [6, 22, 28, 38] {
        let mut altered = input.clone();
        altered[index] ^= 1;
        output.fill(0xaa);
        assert_eq!(arp::reply(&altered, endpoint(), &mut output), Ok(None));
        assert_eq!(output, [0xaa; 64]);
    }
}

#[test]
fn arp_and_routing_reject_all_truncated_inputs_and_short_outputs_atomically() {
    let arp = decode(include_str!("fixtures/arp-request.hex"));
    let ip = decode(include_str!("fixtures/routed-input.hex"));
    let mut output = [0xaa; 64];
    for length in 0..arp.len() {
        assert!(arp::reply(&arp[..length], endpoint(), &mut output).is_err());
    }
    for length in 0..ip.len() {
        assert!(routed::rewrite(&ip[..length], route(), &mut output).is_err());
    }
    for length in 0..42 {
        assert_eq!(
            arp::reply(&arp, endpoint(), &mut output[..length]),
            Err(Error::Capacity { required: 42 })
        );
    }
    for length in 0..54 {
        assert_eq!(
            routed::rewrite(&ip, route(), &mut output[..length]),
            Err(Error::Capacity { required: 54 })
        );
    }
    assert_eq!(output, [0xaa; 64]);
}

#[test]
fn routing_preserves_transport_and_matches_independent_ttl_and_checksum_fixture() {
    let input = decode(include_str!("fixtures/routed-input.hex"));
    let mut output = [0xaa; 64];
    let length = routed::rewrite(&input, route(), &mut output).unwrap();
    assert_eq!(
        &output[..length],
        decode(include_str!("fixtures/routed-output.hex"))
    );
    assert_eq!(&output[34..length], &input[34..]);
    assert_eq!(&output[length..], &[0xaa; 10]);
    assert_eq!(common::checksum(&output[14..34]), 0);
}

#[test]
fn spoofed_routes_bad_ttl_fragments_and_transport_lengths_do_not_change_output() {
    let input = decode(include_str!("fixtures/routed-input.hex"));
    for (index, value) in [
        (0, 4),
        (6, 4),
        (26, 11),
        (30, 193),
        (22, 1),
        (20, 0x20),
        (46, 0x40),
    ] {
        let mut altered = input.clone();
        altered[index] = value;
        common::repair_ip(&mut altered);
        let mut output = [0xaa; 64];
        assert!(routed::rewrite(&altered, route(), &mut output).is_err());
        assert_eq!(output, [0xaa; 64]);
    }
}
