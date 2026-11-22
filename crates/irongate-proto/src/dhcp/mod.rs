//! Bounded DHCPv4 request views and direct-client broadcast OFFER encoding.

mod offer;
mod options;
mod request;

pub use offer::{Offer, encode_offer};
pub use options::{OptionView, Options};
pub use request::{Request, parse};

/// Maximum DHCP payload size accepted independently of outer framing.
pub const MAX_MESSAGE_BYTES: usize = 1472;
/// Maximum option bytes, including PAD and END.
pub const MAX_OPTION_BYTES: usize = 1024;
/// Maximum non-PAD options accepted in one message.
pub const MAX_OPTIONS: usize = 64;

/// Message types defined in the original DHCPv4 protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Client discovery.
    Discover = 1,
    /// Server offer.
    Offer = 2,
    /// Client request.
    Request = 3,
    /// Client decline.
    Decline = 4,
    /// Server acknowledgement.
    Ack = 5,
    /// Server negative acknowledgement.
    Nak = 6,
    /// Client release.
    Release = 7,
    /// Client parameter request for an existing address.
    Inform = 8,
}

impl MessageType {
    pub(crate) fn from_byte(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::Discover,
            2 => Self::Offer,
            3 => Self::Request,
            4 => Self::Decline,
            5 => Self::Ack,
            6 => Self::Nak,
            7 => Self::Release,
            8 => Self::Inform,
            _ => return None,
        })
    }
}
