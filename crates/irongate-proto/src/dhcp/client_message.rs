//! Field combinations from RFC 2131 table 4, without inferring client authority.

use super::{MessageType, Request};
use crate::{Error, Layer};

/// Direct-client lease operation. INFORM and relays are outside this slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    /// Discovery; requested address is a hint.
    Discover {
        /// Optional address hint (option 50).
        requested: Option<[u8; 4]>,
    },
    /// Selection of a particular server's offer.
    Selecting {
        /// Selected server (option 54).
        server: [u8; 4],
        /// Requested address (option 50).
        address: [u8; 4],
    },
    /// Validation of a cached address after reboot.
    InitReboot {
        /// Cached address (option 50).
        address: [u8; 4],
    },
    /// Renewal or rebinding of an existing address.
    Renew {
        /// Existing address (ciaddr).
        address: [u8; 4],
    },
    /// Return of an existing binding.
    Release {
        /// Owning server (option 54).
        server: [u8; 4],
        /// Address returned (ciaddr).
        address: [u8; 4],
    },
    /// Report of an address conflict.
    Decline {
        /// Owning server (option 54).
        server: [u8; 4],
        /// Conflicting address (option 50).
        address: [u8; 4],
    },
}

impl Request<'_> {
    /// Classify a supported lease command, rejecting inconsistent ciaddr/options.
    /// This does not validate Ethernet/IP routing, reserve an address, or prove identity.
    pub fn lease_message(&self) -> Result<ClientMessage, Error> {
        let requested = self
            .option(50)
            .map(|v| v.try_into().expect("validated option"));
        let server = self
            .option(54)
            .map(|v| v.try_into().expect("validated option"));
        let ciaddr = self.client_ip();
        let initial = ciaddr == [0; 4];
        let message = match (self.message_type(), initial, requested, server) {
            (MessageType::Discover, true, requested, None) => ClientMessage::Discover { requested },
            (MessageType::Request, true, Some(address), Some(server)) => {
                ClientMessage::Selecting { server, address }
            }
            (MessageType::Request, true, Some(address), None) => {
                ClientMessage::InitReboot { address }
            }
            (MessageType::Request, false, None, None) => ClientMessage::Renew { address: ciaddr },
            (MessageType::Release, false, None, Some(server)) => ClientMessage::Release {
                server,
                address: ciaddr,
            },
            (MessageType::Decline, true, Some(address), Some(server)) => {
                ClientMessage::Decline { server, address }
            }
            (MessageType::Inform, ..) => return Err(Error::Unsupported(Layer::Dhcp)),
            _ => return Err(Error::Malformed(Layer::Dhcp)),
        };
        Ok(message)
    }
}
