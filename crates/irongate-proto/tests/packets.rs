//! Layer bounds, checksum behavior and borrowed views using synthetic frames.

mod common;
use common::{DISCOVER, TAGGED, decode, repair_ip};
use irongate_proto::{Error, Layer, MAX_FRAME_BYTES, dhcp::MessageType, parse_request};

#[test]
fn discover_and_vlan_identity_borrow_original_input() {
    for (hex, vlan) in [(DISCOVER, None), (TAGGED, Some(100))] {
        let frame = decode(hex);
        let request = parse_request(&frame).unwrap();
        assert_eq!(request.ethernet().vlan.map(|tag| tag.id()), vlan);
        if let Some(tag) = request.ethernet().vlan {
            assert_eq!(tag.tci(), 0xa064);
        }
        assert_eq!(request.dhcp().message_type(), MessageType::Discover);
        assert_eq!(request.dhcp().transaction_id(), 0x12345678);
        assert_eq!(request.dhcp().client_mac(), [2, 0, 0, 0, 0, 2]);
        assert!(request.dhcp().broadcast_requested());
        assert!(request.ip().udp_checksum_present);
        let option = request.dhcp().option(53).unwrap();
        assert_eq!(option, &[1]);
        let offset = if vlan.is_some() { 288 } else { 284 };
        assert_eq!(option.as_ptr(), frame[offset..].as_ptr());
    }
}

#[test]
fn every_truncated_prefix_is_rejected_without_panicking() {
    for hex in [DISCOVER, TAGGED] {
        let frame = decode(hex);
        for end in 0..frame.len() {
            assert!(parse_request(&frame[..end]).is_err(), "prefix {end}");
        }
    }
}

#[test]
fn corrupt_ip_and_udp_checksums_are_rejected() {
    let mut frame = decode(DISCOVER);
    frame[22] ^= 1;
    assert_eq!(
        parse_request(&frame).unwrap_err(),
        Error::Checksum(Layer::Ipv4)
    );
    let mut frame = decode(DISCOVER);
    frame[100] ^= 1;
    assert_eq!(
        parse_request(&frame).unwrap_err(),
        Error::Checksum(Layer::Udp)
    );
}

#[test]
fn zero_udp_checksum_is_accepted_with_explicit_missing_evidence() {
    let frame = decode(include_str!("fixtures/discover-zero-checksum.hex"));
    assert!(!parse_request(&frame).unwrap().ip().udp_checksum_present);
}

#[test]
fn ethernet_padding_is_not_dhcp_input_and_frame_size_is_bounded() {
    let mut frame = decode(DISCOVER);
    frame.extend_from_slice(&[0xaa; 20]);
    assert!(parse_request(&frame).is_ok());
    frame.resize(MAX_FRAME_BYTES + 1, 0);
    assert_eq!(
        parse_request(&frame).unwrap_err(),
        Error::LimitExceeded(Layer::Ethernet)
    );
}

#[test]
fn unsupported_fragmentation_options_protocols_and_tags_are_explicit() {
    for (offset, value, layer) in [
        (20, 0x20, Layer::Ipv4),
        (21, 1, Layer::Ipv4),
        (14, 0x46, Layer::Ipv4),
        (23, 6, Layer::Ipv4),
        (12, 0x86, Layer::Ethernet),
    ] {
        let mut frame = decode(DISCOVER);
        frame[offset] = value;
        assert_eq!(
            parse_request(&frame).unwrap_err(),
            Error::Unsupported(layer)
        );
    }
    let mut frame = decode(TAGGED);
    frame[16..18].copy_from_slice(&0x8100u16.to_be_bytes());
    assert_eq!(
        parse_request(&frame).unwrap_err(),
        Error::Unsupported(Layer::Ethernet)
    );
    for tci in [0u16, 4095] {
        frame[14..16].copy_from_slice(&tci.to_be_bytes());
        assert_eq!(
            parse_request(&frame).unwrap_err(),
            Error::Unsupported(Layer::Ethernet)
        );
    }
}

#[test]
fn inconsistent_lengths_invalid_flags_and_wrong_ports_fail() {
    for (offset, value, layer) in [
        (16, 0, Layer::Ipv4),
        (20, 0x80, Layer::Ipv4),
        (22, 0, Layer::Ipv4),
        (38, 0, Layer::Udp),
    ] {
        let mut frame = decode(DISCOVER);
        frame[offset] = value;
        if offset == 16 {
            frame[17] = 27;
        }
        if offset == 38 {
            frame[39] = 7;
        }
        repair_ip(&mut frame);
        assert_eq!(parse_request(&frame).unwrap_err(), Error::Malformed(layer));
    }
    let mut frame = decode(DISCOVER);
    frame[34..36].copy_from_slice(&67u16.to_be_bytes());
    frame[40..42].fill(0);
    assert_eq!(
        parse_request(&frame).unwrap_err(),
        Error::Unsupported(Layer::Udp)
    );
}

#[test]
fn each_byte_mutation_is_panic_free() {
    let original = decode(TAGGED);
    for index in 0..original.len() {
        for value in [0, 1, 127, 255] {
            let mut frame = original.clone();
            frame[index] = value;
            let _ = parse_request(&frame);
        }
    }
}
