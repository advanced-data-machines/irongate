//! A bounded experiment, not lease selection or an authorization decision.

use super::MessageType;
use crate::{Error, RequestFrame, checksum};

/// Caller-selected OFFER parameters. Deployment/subnet policy belongs to the caller.
#[derive(Clone, Copy, Debug)]
pub struct Offer {
    /// Server Ethernet address.
    pub server_mac: [u8; 6],
    /// Server IPv4 address and DHCP server identifier.
    pub server_ip: [u8; 4],
    /// Offered client address. No lease is reserved by this encoder.
    pub offered_ip: [u8; 4],
    /// Subnet mask option.
    pub subnet_mask: [u8; 4],
    /// Single router option.
    pub router: [u8; 4],
    /// Single DNS server option.
    pub dns: [u8; 4],
    /// Nonzero lease duration in seconds.
    pub lease_seconds: u32,
}

/// Encode a broadcast OFFER, echoing transaction/client identity and inline VLAN TCI.
///
/// Only DISCOVER with zero source IP and ciaddr is supported. Client identifier
/// (61) is echoed when present. Other requested options are outside this pilot.
/// All checks happen before writing; errors leave output unchanged. The returned
/// length excludes unused output capacity and Ethernet FCS. IPv4/UDP checksums
/// are software-computed; no offload assumptions are made.
pub fn encode_offer(
    request: &RequestFrame<'_>,
    offer: &Offer,
    output: &mut [u8],
) -> Result<usize, Error> {
    if request.dhcp.message_type() != MessageType::Discover
        || request.dhcp.client_ip() != [0; 4]
        || request.ip.source != [0; 4]
    {
        return Err(Error::NotDiscover);
    }
    if offer.lease_seconds == 0
        || offer.server_mac == [0; 6]
        || offer.server_mac[0] & 1 != 0
        || !unicast(offer.server_ip)
        || !unicast(offer.offered_ip)
    {
        return Err(Error::InvalidOffer);
    }
    let client_id = request.dhcp.option(61);
    let dhcp_length = (240 + 34 + client_id.map_or(0, |id| id.len() + 2)).max(300);
    let link_length = if request.ethernet.vlan.is_some() {
        18
    } else {
        14
    };
    let required = link_length + 28 + dhcp_length;
    if output.len() < required {
        return Err(Error::Capacity { required });
    }
    let frame = &mut output[..required];
    frame.fill(0);
    frame[..6].fill(255);
    frame[6..12].copy_from_slice(&offer.server_mac);
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
    ip[12..16].copy_from_slice(&offer.server_ip);
    ip[16..20].fill(255);
    let ip_checksum = checksum::finish(checksum::sum(&ip[..20]));
    ip[10..12].copy_from_slice(&ip_checksum.to_be_bytes());
    let udp = &mut ip[20..];
    udp[..2].copy_from_slice(&67u16.to_be_bytes());
    udp[2..4].copy_from_slice(&68u16.to_be_bytes());
    udp[4..6].copy_from_slice(&((8 + dhcp_length) as u16).to_be_bytes());
    let message = &mut udp[8..];
    message[..3].copy_from_slice(&[2, 1, 6]);
    message[4..8].copy_from_slice(&request.dhcp.bytes[4..8]);
    message[10..12].copy_from_slice(&request.dhcp.bytes[10..12]);
    message[16..20].copy_from_slice(&offer.offered_ip);
    message[28..34].copy_from_slice(&request.dhcp.client_mac());
    message[236..240].copy_from_slice(&[99, 130, 83, 99]);
    let mut cursor = 240;
    write_option(message, &mut cursor, 53, &[2]);
    write_option(message, &mut cursor, 54, &offer.server_ip);
    write_option(message, &mut cursor, 51, &offer.lease_seconds.to_be_bytes());
    write_option(message, &mut cursor, 1, &offer.subnet_mask);
    write_option(message, &mut cursor, 3, &offer.router);
    write_option(message, &mut cursor, 6, &offer.dns);
    if let Some(id) = client_id {
        write_option(message, &mut cursor, 61, id);
    }
    message[cursor] = 255;
    let udp_checksum = match checksum::udp(offer.server_ip, [255; 4], udp) {
        0 => 0xffff,
        value => value,
    };
    udp[6..8].copy_from_slice(&udp_checksum.to_be_bytes());
    Ok(required)
}

fn write_option(output: &mut [u8], cursor: &mut usize, code: u8, value: &[u8]) {
    let start = *cursor;
    output[start] = code;
    output[start + 1] = value.len() as u8;
    output[start + 2..start + 2 + value.len()].copy_from_slice(value);
    *cursor += value.len() + 2;
}

fn unicast(address: [u8; 4]) -> bool {
    address[0] != 0 && address[0] != 127 && address[0] < 224
}
