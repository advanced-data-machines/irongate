//! Largest echoed identity must fit ACK/NAK arithmetic without partial writes.
mod common;

use irongate_proto::{
    Error,
    dhcp::{self, Server},
    ipv4_udp, parse_request,
};

#[test]
fn maximum_client_identifier_has_exact_ack_and_nak_output_bounds() {
    let mut options = vec![
        53, 1, 3, 50, 4, 192, 0, 2, 100, 54, 4, 192, 0, 2, 1, 61, 255,
    ];
    options.extend(0..255u8);
    options.push(255);
    let payload = common::payload(&options);
    let mut frame = common::decode(common::DISCOVER)[..42].to_vec();
    frame.extend_from_slice(&payload);
    let ip_length = (frame.len() - 14) as u16;
    frame[16..18].copy_from_slice(&ip_length.to_be_bytes());
    frame[38..40].copy_from_slice(&(ip_length - 20).to_be_bytes());
    frame[40..42].fill(0);
    common::repair_ip(&mut frame);
    let request = parse_request(&frame).unwrap();
    let lease = common::offer();
    let server = Server {
        mac: lease.server_mac,
        ip: lease.server_ip,
    };
    let mut output = [0xaa; 1024];
    for (required, nak) in [(573, false), (549, true)] {
        for size in 0..required {
            let result = if nak {
                dhcp::encode_nak(&request, server, &mut output[..size])
            } else {
                dhcp::encode_ack(&request, &lease, &mut output[..size])
            };
            assert_eq!(result, Err(Error::Capacity { required }));
            assert_eq!(output, [0xaa; 1024]);
        }
        let result = if nak {
            dhcp::encode_nak(&request, server, &mut output[..required])
        } else {
            dhcp::encode_ack(&request, &lease, &mut output[..required])
        };
        assert_eq!(result.unwrap(), required);
        let id_start = if nak { 293 } else { 317 };
        assert_eq!(
            &output[id_start..required - 1],
            &(0..255u8).collect::<Vec<_>>()
        );
        assert_eq!(output[required - 1], 255);
        assert!(output[required..].iter().all(|byte| *byte == 0xaa));
        assert!(
            ipv4_udp::parse(&output[14..required])
                .unwrap()
                .udp_checksum_present
        );
        output.fill(0xaa);
    }
}
