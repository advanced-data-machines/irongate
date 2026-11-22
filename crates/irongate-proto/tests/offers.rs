//! Encoder checks use independently generated wire fixtures, not just round trips.

mod common;
use common::{DISCOVER, TAGGED, decode, offer};
use irongate_proto::{
    Error,
    dhcp::{self, encode_offer},
    ethernet, ipv4_udp, parse_request,
};

#[test]
fn offers_match_independent_wire_fixtures_including_vlan_and_client_id() {
    for (input, expected) in [
        (DISCOVER, include_str!("fixtures/offer.hex")),
        (TAGGED, include_str!("fixtures/offer-vlan-client-id.hex")),
    ] {
        let input = decode(input);
        let request = parse_request(&input).unwrap();
        let mut output = [0xaa; 1024];
        let length = encode_offer(&request, &offer(), &mut output).unwrap();
        assert_eq!(&output[..length], decode(expected));
        assert!(output[length..].iter().all(|byte| *byte == 0xaa));
        let link = ethernet::parse(&output[..length]).unwrap();
        let datagram = ipv4_udp::parse(link.payload).unwrap();
        assert_eq!((datagram.source_port, datagram.destination_port), (67, 68));
        assert!(datagram.udp_checksum_present);
        assert_eq!(datagram.destination, [255; 4]);
    }
}

#[test]
fn every_short_buffer_fails_without_partial_output_and_exact_capacity_works() {
    let input = decode(TAGGED);
    let request = parse_request(&input).unwrap();
    let required = 346;
    for capacity in 0..required {
        let mut output = vec![0xaa; capacity];
        assert_eq!(
            encode_offer(&request, &offer(), &mut output),
            Err(Error::Capacity { required })
        );
        assert!(output.iter().all(|byte| *byte == 0xaa));
    }
    assert_eq!(
        encode_offer(&request, &offer(), &mut [0; 346]).unwrap(),
        required
    );
}

#[test]
fn invalid_offer_and_non_discover_do_not_modify_output() {
    let input = decode(include_str!("fixtures/request.hex"));
    let request = parse_request(&input).unwrap();
    let mut output = [0xaa; 1024];
    assert_eq!(
        encode_offer(&request, &offer(), &mut output),
        Err(Error::NotDiscover)
    );
    let input = decode(DISCOVER);
    let request = parse_request(&input).unwrap();
    for kind in 0..4 {
        let mut offer = offer();
        match kind {
            0 => offer.lease_seconds = 0,
            1 => offer.server_ip = [0; 4],
            2 => offer.offered_ip = [255; 4],
            _ => offer.server_mac = [255; 6],
        }
        assert_eq!(
            encode_offer(&request, &offer, &mut output),
            Err(Error::InvalidOffer)
        );
    }
    assert_eq!(output, [0xaa; 1024]);
}

#[test]
fn largest_client_identifier_is_echoed_with_odd_length_checksum_and_bounded_output() {
    let mut options = vec![53, 1, 1, 61, 255];
    options.extend(0..255u8);
    options.push(255);
    let message = common::payload(&options);
    assert!(dhcp::parse(&message).is_ok());
    let mut frame = decode(DISCOVER)[..42].to_vec();
    frame.extend_from_slice(&message);
    let ip_length = (frame.len() - 14) as u16;
    let udp_length = ip_length - 20;
    frame[16..18].copy_from_slice(&ip_length.to_be_bytes());
    frame[38..40].copy_from_slice(&udp_length.to_be_bytes());
    frame[40..42].fill(0);
    common::repair_ip(&mut frame);
    let request = parse_request(&frame).unwrap();
    let mut output = [0xaa; 1024];
    let length = encode_offer(&request, &offer(), &mut output).unwrap();
    assert_eq!(length, 573); // 14 + 28 + (240 + 34 + 257).
    assert_eq!(&output[317..572], &(0..255u8).collect::<Vec<_>>());
    assert_eq!(output[572], 255);
    let datagram = ipv4_udp::parse(&output[14..length]).unwrap();
    assert!(datagram.udp_checksum_present);
}

#[test]
fn computed_zero_udp_checksum_is_transmitted_as_ffff() {
    let input = decode(DISCOVER);
    let request = parse_request(&input).unwrap();
    let mut settings = offer();
    let mut output = [0; 1024];
    encode_offer(&request, &settings, &mut output).unwrap();
    // Add the original checksum to a payload word, with one's-complement carry.
    // This makes the checksum before wire substitution exactly zero.
    // DNS bytes land at an odd offset in this option layout, so their checksum
    // contribution has the opposite byte order from the address's final word.
    let old = u32::from(u16::from_be_bytes([settings.dns[3], settings.dns[2]]));
    let sum = old + u32::from(u16::from_be_bytes([output[40], output[41]]));
    let adjusted = ((sum & 65535) + (sum >> 16)) as u16;
    settings.dns[2..4].copy_from_slice(&adjusted.to_le_bytes());
    let length = encode_offer(&request, &settings, &mut output).unwrap();
    assert_eq!(&output[40..42], &[255, 255]);
    let mut pseudo = Vec::from(settings.server_ip);
    pseudo.extend_from_slice(&[255, 255, 255, 255, 0, 17]);
    pseudo.extend_from_slice(&((length - 34) as u16).to_be_bytes());
    pseudo.extend_from_slice(&output[34..length]);
    assert_eq!(common::checksum(&pseudo), 0);
}
