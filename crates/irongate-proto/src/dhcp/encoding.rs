//! Shared Ethernet/IPv4/UDP/BOOTP writer, after all fallible validation.

use super::{MessageType, Offer, Server};
use crate::{Error, RequestFrame, checksum};

pub(super) fn unicast(address: [u8; 4]) -> bool {
    address[0] != 0 && address[0] != 127 && address[0] < 224
}

pub(super) fn valid_server(server: Server) -> bool {
    server.mac != [0; 6] && server.mac[0] & 1 == 0 && unicast(server.ip)
}

pub(super) fn valid_lease(lease: &Offer) -> bool {
    valid_server(Server {
        mac: lease.server_mac,
        ip: lease.server_ip,
    }) && unicast(lease.offered_ip)
        && lease.lease_seconds != 0
}

pub(super) fn encode(
    request: &RequestFrame<'_>,
    server: Server,
    kind: MessageType,
    lease: Option<&Offer>,
    broadcast: bool,
    output: &mut [u8],
) -> Result<usize, Error> {
    let id = request.dhcp.option(61);
    let option_bytes = if lease.is_some() { 34 } else { 10 };
    let dhcp_length = (240 + option_bytes + id.map_or(0, |id| id.len() + 2)).max(300);
    let link_length = if request.ethernet.vlan.is_some() {
        18
    } else {
        14
    };
    let required = link_length + 28 + dhcp_length;
    if output.len() < required {
        return Err(Error::Capacity { required });
    }
    let assigned = lease.map_or([0; 4], |lease| lease.offered_ip);
    let destination = if broadcast {
        [255; 4]
    } else if request.dhcp.client_ip() != [0; 4] {
        request.dhcp.client_ip()
    } else {
        assigned
    };
    let frame = &mut output[..required];
    frame.fill(0);
    frame[..6].copy_from_slice(&if broadcast {
        [255; 6]
    } else {
        request.dhcp.client_mac()
    });
    frame[6..12].copy_from_slice(&server.mac);
    if let Some(tag) = request.ethernet.vlan {
        frame[12..14].copy_from_slice(&0x8100u16.to_be_bytes());
        frame[14..16].copy_from_slice(&tag.tci().to_be_bytes());
    }
    frame[link_length - 2..link_length].copy_from_slice(&0x0800u16.to_be_bytes());
    let ip = &mut frame[link_length..];
    ip[0] = 0x45;
    ip[2..4].copy_from_slice(&((28 + dhcp_length) as u16).to_be_bytes());
    ip[8] = 64;
    ip[9] = 17;
    ip[12..16].copy_from_slice(&server.ip);
    ip[16..20].copy_from_slice(&destination);
    let value = checksum::finish(checksum::sum(&ip[..20]));
    ip[10..12].copy_from_slice(&value.to_be_bytes());
    let udp = &mut ip[20..];
    udp[..2].copy_from_slice(&67u16.to_be_bytes());
    udp[2..4].copy_from_slice(&68u16.to_be_bytes());
    udp[4..6].copy_from_slice(&((8 + dhcp_length) as u16).to_be_bytes());
    let message = &mut udp[8..];
    message[..3].copy_from_slice(&[2, 1, 6]);
    message[4..8].copy_from_slice(&request.dhcp.bytes[4..8]);
    message[10..12].copy_from_slice(&request.dhcp.bytes[10..12]);
    if kind == MessageType::Ack {
        message[12..16].copy_from_slice(&request.dhcp.client_ip());
    }
    message[16..20].copy_from_slice(&assigned);
    message[28..34].copy_from_slice(&request.dhcp.client_mac());
    message[236..240].copy_from_slice(&[99, 130, 83, 99]);
    let mut cursor = 240;
    write_option(message, &mut cursor, 53, &[kind as u8]);
    write_option(message, &mut cursor, 54, &server.ip);
    if let Some(lease) = lease {
        write_option(message, &mut cursor, 51, &lease.lease_seconds.to_be_bytes());
        write_option(message, &mut cursor, 1, &lease.subnet_mask);
        write_option(message, &mut cursor, 3, &lease.router);
        write_option(message, &mut cursor, 6, &lease.dns);
    }
    if let Some(id) = id {
        write_option(message, &mut cursor, 61, id);
    }
    message[cursor] = 255;
    let value = match checksum::udp(server.ip, destination, udp) {
        0 => 0xffff,
        value => value,
    };
    udp[6..8].copy_from_slice(&value.to_be_bytes());
    Ok(required)
}

fn write_option(output: &mut [u8], cursor: &mut usize, code: u8, value: &[u8]) {
    let start = *cursor;
    output[start] = code;
    output[start + 1] = value.len() as u8;
    output[start + 2..start + 2 + value.len()].copy_from_slice(value);
    *cursor += value.len() + 2;
}
