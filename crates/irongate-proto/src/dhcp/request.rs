//! Request invariants belong to the borrowed view's constructor.

use super::{MAX_MESSAGE_BYTES, MessageType, Options, options};
use crate::{Error, Layer};

/// A supported direct Ethernet client request; fields still require service policy.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub(super) bytes: &'a [u8],
    message_type: MessageType,
}

impl<'a> Request<'a> {
    /// Transaction correlation identifier; not an authentication credential.
    pub fn transaction_id(&self) -> u32 {
        u32::from_be_bytes(self.bytes[4..8].try_into().expect("validated fixed header"))
    }
    /// Ethernet client address from chaddr.
    pub fn client_mac(&self) -> [u8; 6] {
        self.bytes[28..34]
            .try_into()
            .expect("validated fixed header")
    }
    /// Existing client IPv4 address (ciaddr).
    pub fn client_ip(&self) -> [u8; 4] {
        self.bytes[12..16]
            .try_into()
            .expect("validated fixed header")
    }
    /// DHCP message type; the pilot accepts client types only.
    pub fn message_type(&self) -> MessageType {
        self.message_type
    }
    /// The client's broadcast response preference.
    pub fn broadcast_requested(&self) -> bool {
        self.bytes[10] & 0x80 != 0
    }
    /// Borrowed, validated options in wire order.
    pub fn options(&self) -> Options<'a> {
        Options {
            bytes: &self.bytes[240..],
        }
    }
    /// First raw value for an option. Unknown repeated options remain separate.
    pub fn option(&self, code: u8) -> Option<&'a [u8]> {
        self.options()
            .find(|option| option.code == code)
            .map(|option| option.value)
    }
}

/// Parse a BOOTREQUEST with Ethernet hardware fields and no relay/overload.
pub fn parse(bytes: &[u8]) -> Result<Request<'_>, Error> {
    let layer = Layer::Dhcp;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(Error::LimitExceeded(layer));
    }
    if bytes.len() < 240 {
        return Err(Error::Truncated(layer));
    }
    if bytes[..3] != [1, 1, 6] {
        return Err(Error::Unsupported(layer));
    }
    if bytes[3] != 0 || bytes[24..28] != [0; 4] {
        return Err(Error::Unsupported(layer));
    }
    if bytes[10] & 0x7f != 0 || bytes[11] != 0 {
        return Err(Error::Malformed(layer));
    }
    if bytes[28..34] == [0; 6] || bytes[28] & 1 != 0 {
        return Err(Error::Malformed(layer));
    }
    if bytes[236..240] != [99, 130, 83, 99] {
        return Err(Error::Malformed(layer));
    }
    let message_type = options::validate(&bytes[240..])?;
    if matches!(
        message_type,
        MessageType::Offer | MessageType::Ack | MessageType::Nak
    ) {
        return Err(Error::Unsupported(layer));
    }
    Ok(Request {
        bytes,
        message_type,
    })
}
