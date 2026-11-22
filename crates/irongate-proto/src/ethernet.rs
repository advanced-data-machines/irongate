//! Ethernet II and at most one inline 802.1Q tag; FCS is not included.

use crate::{Error, Layer, MAX_FRAME_BYTES};

/// One inline VLAN tag; preserve PCP and DEI as well as the VLAN identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VlanTag(u16);

impl VlanTag {
    /// Network-order TCI interpreted as a host integer.
    pub fn tci(self) -> u16 {
        self.0
    }
    /// VLAN identifier; the pilot accepts 1 through 4094.
    pub fn id(self) -> u16 {
        self.0 & 0x0fff
    }
}

/// Borrowed Ethernet payload and copied fixed-size addressing metadata.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// Source address.
    pub source: [u8; 6],
    /// Destination address.
    pub destination: [u8; 6],
    /// Inline tag, if present. Hardware-stripped tags require a future adapter.
    pub vlan: Option<VlanTag>,
    /// IPv4 datagram plus possible Ethernet padding.
    pub payload: &'a [u8],
}

/// Parse the pilot's Ethernet/IPv4 form without copying the payload.
pub fn parse(bytes: &[u8]) -> Result<Frame<'_>, Error> {
    let layer = Layer::Ethernet;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(Error::LimitExceeded(layer));
    }
    if bytes.len() < 14 {
        return Err(Error::Truncated(layer));
    }
    let mut ether_type = u16::from_be_bytes([bytes[12], bytes[13]]);
    let mut offset = 14;
    let vlan = if ether_type == 0x8100 {
        if bytes.len() < 18 {
            return Err(Error::Truncated(layer));
        }
        let tag = VlanTag(u16::from_be_bytes([bytes[14], bytes[15]]));
        if matches!(tag.id(), 0 | 4095) {
            return Err(Error::Unsupported(layer));
        }
        ether_type = u16::from_be_bytes([bytes[16], bytes[17]]);
        offset = 18;
        Some(tag)
    } else {
        None
    };
    if ether_type != 0x0800 {
        return Err(Error::Unsupported(layer));
    }
    Ok(Frame {
        destination: bytes[..6].try_into().expect("checked Ethernet length"),
        source: bytes[6..12].try_into().expect("checked Ethernet length"),
        vlan,
        payload: &bytes[offset..],
    })
}
