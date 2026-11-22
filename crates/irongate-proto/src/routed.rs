//! One explicit untagged IPv4 route; no NAT, fragments or transport stack.

use crate::{Error, Layer, MAX_FRAME_BYTES, checksum};

/// Fixed peer and route checks for the disposable lab, not a routing table.
#[derive(Clone, Copy, Debug)]
pub struct Route {
    /// Required incoming source MAC.
    pub source_mac: [u8; 6],
    /// Required incoming destination MAC (our receiving interface).
    pub gateway_mac: [u8; 6],
    /// Required source IP; scope/lease validation remains the policy owner's job.
    pub source_ip: [u8; 4],
    /// Exact permitted destination IP.
    pub destination_ip: [u8; 4],
    /// Sending interface MAC.
    pub output_mac: [u8; 6],
    /// Explicit next-hop MAC; no unbounded neighbor cache.
    pub next_hop_mac: [u8; 6],
}

/// Validate route identity, decrement TTL and rebuild Ethernet/IP checksum in
/// caller-owned output. Transport bytes/checksums remain unchanged (no NAT).
/// The caller must independently authorize the current session before sending.
pub fn rewrite(frame: &[u8], route: Route, output: &mut [u8]) -> Result<usize, Error> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err(Error::LimitExceeded(Layer::Ethernet));
    }
    if frame.len() < 34 {
        return Err(Error::Truncated(Layer::Ipv4));
    }
    if frame[..6] != route.gateway_mac
        || frame[6..12] != route.source_mac
        || frame[12..14] != [8, 0]
    {
        return Err(Error::Unsupported(Layer::Ethernet));
    }
    let ip = &frame[14..];
    if ip[0] != 0x45 || !matches!(ip[9], 1 | 6 | 17) {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    let length = usize::from(u16::from_be_bytes([ip[2], ip[3]]));
    if length < 20 || length > ip.len() {
        return Err(Error::Truncated(Layer::Ipv4));
    }
    let fragment = u16::from_be_bytes([ip[6], ip[7]]);
    if fragment & !0x4000 != 0 || ip[8] <= 1 {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    if checksum::finish(checksum::sum(&ip[..20])) != 0 {
        return Err(Error::Checksum(Layer::Ipv4));
    }
    if ip[12..16] != route.source_ip || ip[16..20] != route.destination_ip {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    let transport = &ip[20..length];
    let valid_transport = match ip[9] {
        1 => transport.len() >= 8,
        6 => {
            transport.len() >= 20
                && transport[12] >> 4 >= 5
                && usize::from(transport[12] >> 4) * 4 <= transport.len()
        }
        17 => {
            transport.len() >= 8
                && usize::from(u16::from_be_bytes([transport[4], transport[5]])) == transport.len()
        }
        _ => false,
    };
    if !valid_transport {
        return Err(Error::Malformed(Layer::Ipv4));
    }
    let required = 14 + length;
    if output.len() < required {
        return Err(Error::Capacity { required });
    }
    let out = &mut output[..required];
    out[..6].copy_from_slice(&route.next_hop_mac);
    out[6..12].copy_from_slice(&route.output_mac);
    out[12..14].copy_from_slice(&[8, 0]);
    out[14..].copy_from_slice(&ip[..length]);
    out[22] -= 1;
    out[24..26].fill(0);
    let value = checksum::finish(checksum::sum(&out[14..34]));
    out[24..26].copy_from_slice(&value.to_be_bytes());
    Ok(required)
}
