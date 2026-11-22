//! Compose borrowed layers without conflating network context and identity.

use crate::{Error, Layer, dhcp, ethernet, ipv4_udp};

/// Validated supported direct-client request. No authentication is implied.
#[derive(Clone, Copy, Debug)]
pub struct RequestFrame<'a> {
    pub(crate) ethernet: ethernet::Frame<'a>,
    pub(crate) ip: ipv4_udp::Datagram<'a>,
    pub(crate) dhcp: dhcp::Request<'a>,
}

impl<'a> RequestFrame<'a> {
    /// Link metadata, including an inline tag when present.
    pub fn ethernet(&self) -> &ethernet::Frame<'a> {
        &self.ethernet
    }
    /// IP/UDP metadata and checksum availability.
    pub fn ip(&self) -> &ipv4_udp::Datagram<'a> {
        &self.ip
    }
    /// Borrowed validated DHCP request.
    pub fn dhcp(&self) -> &dhcp::Request<'a> {
        &self.dhcp
    }
}

/// Validate one direct-client frame (UDP 68 to 67), without allocation.
pub fn parse_request(bytes: &[u8]) -> Result<RequestFrame<'_>, Error> {
    let ethernet = ethernet::parse(bytes)?;
    let ip = ipv4_udp::parse(ethernet.payload)?;
    if ip.source_port != 68 || ip.destination_port != 67 {
        return Err(Error::Unsupported(Layer::Udp));
    }
    let dhcp = dhcp::parse(ip.payload)?;
    Ok(RequestFrame { ethernet, ip, dhcp })
}
