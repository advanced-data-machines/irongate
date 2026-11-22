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
    /// Decode a lease command with matching direct-client Ethernet/IP metadata.
    /// Network selection must still come from a trusted receiving interface/VLAN
    /// mapping, never an identifier supplied by the client.
    pub fn lease_message(&self) -> Result<dhcp::ClientMessage, Error> {
        if self.ethernet.source != self.dhcp.client_mac() {
            return Err(Error::Malformed(Layer::Ethernet));
        }
        if self.ip.source != self.dhcp.client_ip() {
            return Err(Error::Malformed(Layer::Ipv4));
        }
        self.dhcp.lease_message()
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
