//! A bounded experiment, not lease selection or an authorization decision.

use super::{MessageType, Server, encoding};
use crate::{Error, RequestFrame};

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
    if !encoding::valid_lease(offer) {
        return Err(Error::InvalidOffer);
    }
    encoding::encode(
        request,
        Server {
            mac: offer.server_mac,
            ip: offer.server_ip,
        },
        MessageType::Offer,
        Some(offer),
        true,
        output,
    )
}
