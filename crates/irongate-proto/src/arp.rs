//! Untagged Ethernet/IPv4 ARP for the isolated one-client lab.

use crate::{Error, Layer, MAX_FRAME_BYTES};

/// Reply only for this local address and expected peer. No neighbor learning.
#[derive(Clone, Copy, Debug)]
pub struct Endpoint {
    /// Local Ethernet address.
    pub mac: [u8; 6],
    /// Local protocol address.
    pub ip: [u8; 4],
    /// Explicitly configured lab peer MAC.
    pub peer_mac: [u8; 6],
    /// Expected sender IP; zero sender IP is also accepted for address probes.
    pub peer_ip: [u8; 4],
}

/// Answer a direct peer's request for our exact IP. Return `None` for another
/// target/opcode/peer. Ethernet padding is accepted; VLAN/relay ARP is excluded.
/// Validate before writing; output is the 42-byte frame without FCS/padding.
pub fn reply(frame: &[u8], endpoint: Endpoint, output: &mut [u8]) -> Result<Option<usize>, Error> {
    if frame.len() > MAX_FRAME_BYTES {
        return Err(Error::LimitExceeded(Layer::Arp));
    }
    if frame.len() < 42 {
        return Err(Error::Truncated(Layer::Arp));
    }
    if frame[12..14] != [8, 6] || frame[14..20] != [0, 1, 8, 0, 6, 4] {
        return Err(Error::Unsupported(Layer::Arp));
    }
    if frame[20..22] != [0, 1]
        || frame[38..42] != endpoint.ip
        || frame[6..12] != endpoint.peer_mac
        || frame[22..28] != endpoint.peer_mac
        || (frame[28..32] != endpoint.peer_ip && frame[28..32] != [0; 4])
        || (frame[..6] != [255; 6] && frame[..6] != endpoint.mac)
    {
        return Ok(None);
    }
    if output.len() < 42 {
        return Err(Error::Capacity { required: 42 });
    }
    let out = &mut output[..42];
    out.fill(0);
    out[..6].copy_from_slice(&endpoint.peer_mac);
    out[6..12].copy_from_slice(&endpoint.mac);
    out[12..22].copy_from_slice(&[8, 6, 0, 1, 8, 0, 6, 4, 0, 2]);
    out[22..28].copy_from_slice(&endpoint.mac);
    out[28..32].copy_from_slice(&endpoint.ip);
    out[32..38].copy_from_slice(&endpoint.peer_mac);
    out[38..42].copy_from_slice(&frame[28..32]);
    Ok(Some(42))
}
