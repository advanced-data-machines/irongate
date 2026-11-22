//! Synthetic RFC-shaped corpus; decoding happens outside measured packet work.

#![allow(dead_code)]

use irongate_proto::dhcp::Offer;

pub const DISCOVER: &str = include_str!("../fixtures/discover.hex");
pub const TAGGED: &str = include_str!("../fixtures/discover-vlan-client-id.hex");

pub fn decode(hex: &str) -> Vec<u8> {
    hex.split_whitespace()
        .flat_map(|line| {
            line.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        })
        .collect()
}

pub fn offer() -> Offer {
    Offer {
        server_mac: [2, 0, 0, 0, 0, 1],
        server_ip: [192, 0, 2, 1],
        offered_ip: [192, 0, 2, 100],
        subnet_mask: [255, 255, 255, 0],
        router: [192, 0, 2, 1],
        dns: [192, 0, 2, 53],
        lease_seconds: 3600,
    }
}

/// Deliberately independent checksum implementation for mutation fixtures.
pub fn checksum(bytes: &[u8]) -> u16 {
    let mut total = 0u64;
    for (index, byte) in bytes.iter().enumerate() {
        total += u64::from(*byte) << if index % 2 == 0 { 8 } else { 0 };
    }
    while total > 65535 {
        total = (total & 65535) + (total >> 16);
    }
    !(total as u16)
}

pub fn repair_ip(frame: &mut [u8]) {
    frame[24..26].fill(0);
    let value = checksum(&frame[14..34]);
    frame[24..26].copy_from_slice(&value.to_be_bytes());
}

pub fn payload(options: &[u8]) -> Vec<u8> {
    let frame = decode(DISCOVER);
    let mut payload = frame[42..282].to_vec();
    payload.extend_from_slice(options);
    payload
}
