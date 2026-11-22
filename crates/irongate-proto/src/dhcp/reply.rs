//! Direct-client ACK/NAK delivery; lease ownership remains the caller's decision.

use super::{ClientMessage, MessageType, Offer, encoding};
use crate::{Error, RequestFrame};

/// DHCP server identity used for negative replies without lease options.
#[derive(Clone, Copy, Debug)]
pub struct Server {
    /// Server Ethernet address.
    pub mac: [u8; 6],
    /// IPv4 address and server identifier.
    pub ip: [u8; 4],
}

/// Encode an ACK for a validated DHCPREQUEST. The caller must first bind the
/// lease. Initial replies honor the broadcast flag; renewal replies unicast to
/// ciaddr. Client identifier and VLAN TCI are echoed. Errors leave output intact.
pub fn encode_ack(
    request: &RequestFrame<'_>,
    lease: &Offer,
    output: &mut [u8],
) -> Result<usize, Error> {
    let message = request_message(request)?;
    let (address, server) = match message {
        ClientMessage::Selecting { address, server } => (address, Some(server)),
        ClientMessage::InitReboot { address } | ClientMessage::Renew { address } => (address, None),
        _ => unreachable!("request_message only returns request forms"),
    };
    if !encoding::valid_lease(lease)
        || address != lease.offered_ip
        || server.is_some_and(|server| server != lease.server_ip)
    {
        return Err(Error::InvalidReply);
    }
    let broadcast = request.dhcp.client_ip() == [0; 4] && request.dhcp.broadcast_requested();
    encoding::encode(
        request,
        Server {
            mac: lease.server_mac,
            ip: lease.server_ip,
        },
        MessageType::Ack,
        Some(lease),
        broadcast,
        output,
    )
}

/// Encode a broadcast NAK with no address, lease, router or DNS assignment.
/// This does not decide whether a negative response is appropriate.
pub fn encode_nak(
    request: &RequestFrame<'_>,
    server: Server,
    output: &mut [u8],
) -> Result<usize, Error> {
    let message = request_message(request)?;
    if !encoding::valid_server(server)
        || matches!(message, ClientMessage::Selecting { server: selected, .. } if selected != server.ip)
    {
        return Err(Error::InvalidReply);
    }
    encoding::encode(request, server, MessageType::Nak, None, true, output)
}

fn request_message(request: &RequestFrame<'_>) -> Result<ClientMessage, Error> {
    if request.dhcp.message_type() != MessageType::Request {
        return Err(Error::NotRequest);
    }
    let message = request.lease_message()?;
    Ok(message)
}
