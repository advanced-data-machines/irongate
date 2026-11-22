//! Independent Python-generated wire expectations, not encoder round trips.

mod common;

use common::{decode, offer};
use irongate_proto::{
    Error, Layer,
    dhcp::{self, ClientMessage, Server},
    parse_request,
};

const SELECTING: &str = include_str!("fixtures/selecting.hex");

#[test]
fn ack_and_nak_match_independent_fixtures_for_broadcast_unicast_and_vlan() {
    for (input, ack, nak) in [
        (
            SELECTING,
            include_str!("fixtures/ack.hex"),
            Some(include_str!("fixtures/nak.hex")),
        ),
        (
            include_str!("fixtures/selecting-vlan-client-id.hex"),
            include_str!("fixtures/ack-vlan-client-id.hex"),
            Some(include_str!("fixtures/nak-vlan-client-id.hex")),
        ),
        (
            include_str!("fixtures/selecting-unicast.hex"),
            include_str!("fixtures/ack-unicast.hex"),
            None,
        ),
        (
            include_str!("fixtures/renew.hex"),
            include_str!("fixtures/ack-renew.hex"),
            Some(include_str!("fixtures/nak-renew.hex")),
        ),
        (
            include_str!("fixtures/renew-vlan-client-id.hex"),
            include_str!("fixtures/ack-renew-vlan-client-id.hex"),
            Some(include_str!("fixtures/nak-renew-vlan-client-id.hex")),
        ),
    ] {
        let bytes = decode(input);
        let request = parse_request(&bytes).unwrap();
        let mut output = [0xaa; 1024];
        let length = dhcp::encode_ack(&request, &offer(), &mut output).unwrap();
        assert_eq!(&output[..length], decode(ack));
        assert!(output[length..].iter().all(|byte| *byte == 0xaa));
        if let Some(nak) = nak {
            let server = Server {
                mac: offer().server_mac,
                ip: offer().server_ip,
            };
            let length = dhcp::encode_nak(&request, server, &mut output).unwrap();
            assert_eq!(&output[..length], decode(nak));
        }
    }
}

#[test]
fn each_request_phase_is_explicit_and_inconsistent_field_combinations_fail() {
    let address = [192, 0, 2, 100];
    let server = [192, 0, 2, 1];
    for (input, expected) in [
        (
            common::DISCOVER,
            ClientMessage::Discover { requested: None },
        ),
        (SELECTING, ClientMessage::Selecting { address, server }),
        (
            include_str!("fixtures/reboot.hex"),
            ClientMessage::InitReboot { address },
        ),
        (
            include_str!("fixtures/renew.hex"),
            ClientMessage::Renew { address },
        ),
        (
            include_str!("fixtures/release.hex"),
            ClientMessage::Release { address, server },
        ),
        (
            include_str!("fixtures/decline.hex"),
            ClientMessage::Decline { address, server },
        ),
    ] {
        let bytes = decode(input);
        assert_eq!(parse_request(&bytes).unwrap().lease_message(), Ok(expected));
    }
    for kind in [1, 3, 4, 7] {
        // A nonzero ciaddr with requested-IP/server-ID cannot be a supported command.
        let mut payload =
            common::payload(&[53, 1, kind, 50, 4, 192, 0, 2, 100, 54, 4, 192, 0, 2, 1, 255]);
        payload[12..16].copy_from_slice(&address);
        assert_eq!(
            dhcp::parse(&payload).unwrap().lease_message(),
            Err(Error::Malformed(Layer::Dhcp))
        );
    }
    let bytes = decode(include_str!("fixtures/request.hex"));
    assert_eq!(
        parse_request(&bytes).unwrap().lease_message(),
        Err(Error::Malformed(Layer::Dhcp))
    );
    let payload = common::payload(&[53, 1, 8, 255]);
    assert_eq!(
        dhcp::parse(&payload).unwrap().lease_message(),
        Err(Error::Unsupported(Layer::Dhcp))
    );
}

#[test]
fn failed_reply_validation_and_short_buffers_leave_output_unchanged() {
    let bytes = decode(SELECTING);
    let request = parse_request(&bytes).unwrap();
    let mut output = [0xaa; 1024];
    let server = Server {
        mac: offer().server_mac,
        ip: offer().server_ip,
    };
    let mut lease = offer();
    lease.offered_ip[3] += 1;
    assert_eq!(
        dhcp::encode_ack(&request, &lease, &mut output),
        Err(Error::InvalidReply)
    );
    let wrong_server = Server {
        ip: [192, 0, 2, 2],
        ..server
    };
    assert_eq!(
        dhcp::encode_nak(&request, wrong_server, &mut output),
        Err(Error::InvalidReply)
    );
    assert_eq!(output, [0xaa; 1024]);
    for length in 0..342 {
        assert_eq!(
            dhcp::encode_ack(&request, &offer(), &mut output[..length]),
            Err(Error::Capacity { required: 342 })
        );
        assert_eq!(
            dhcp::encode_nak(&request, server, &mut output[..length]),
            Err(Error::Capacity { required: 342 })
        );
        assert_eq!(output, [0xaa; 1024]);
    }
    let bytes = decode(common::DISCOVER);
    let request = parse_request(&bytes).unwrap();
    assert_eq!(
        dhcp::encode_ack(&request, &offer(), &mut output),
        Err(Error::NotRequest)
    );
    assert_eq!(
        dhcp::encode_nak(&request, server, &mut output),
        Err(Error::NotRequest)
    );
    assert_eq!(output, [0xaa; 1024]);
}

#[test]
fn direct_client_metadata_must_match_before_a_lease_command_is_accepted() {
    let mut bytes = decode(SELECTING);
    // Disable optional UDP checksum so mutations test command validation directly.
    bytes[40..42].fill(0);
    bytes[6] = 4;
    assert_eq!(
        parse_request(&bytes).unwrap().lease_message(),
        Err(Error::Malformed(Layer::Ethernet))
    );
    bytes[6] = 2;
    bytes[26..30].copy_from_slice(&[192, 0, 2, 100]);
    common::repair_ip(&mut bytes);
    let request = parse_request(&bytes).unwrap();
    let mut output = [0xaa; 1024];
    assert_eq!(
        dhcp::encode_ack(&request, &offer(), &mut output),
        Err(Error::Malformed(Layer::Ipv4))
    );
    assert_eq!(output, [0xaa; 1024]);
}
