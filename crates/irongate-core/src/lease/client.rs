//! Inline bounded client identities for address ownership records.

use super::Error;

/// Owner-assigned ingress network identity, including interface/VLAN context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkId(pub u32);

/// Ethernet fallback or opaque DHCP option 61; these namespaces are distinct.
#[derive(Clone, Copy)]
pub struct ClientIdentity {
    bytes: [u8; 255],
    length: u8,
    option_61: bool,
}

impl PartialEq for ClientIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.option_61 == other.option_61 && self.bytes() == other.bytes()
    }
}
impl Eq for ClientIdentity {}

impl ClientIdentity {
    /// Hardware identity when the client did not provide option 61.
    pub fn ethernet(mac: [u8; 6]) -> Result<Self, Error> {
        if mac == [0; 6] || mac[0] & 1 != 0 {
            return Err(Error::InvalidClient);
        }
        let mut identity = Self {
            bytes: [0; 255],
            length: 6,
            option_61: false,
        };
        identity.bytes[..6].copy_from_slice(&mac);
        Ok(identity)
    }

    /// Preserve the complete 2–255 byte client identifier, including type byte.
    pub fn option_61(value: &[u8]) -> Result<Self, Error> {
        if !(2..=255).contains(&value.len()) {
            return Err(Error::InvalidClient);
        }
        let mut identity = Self {
            bytes: [0; 255],
            length: value.len() as u8,
            option_61: true,
        };
        identity.bytes[..value.len()].copy_from_slice(value);
        Ok(identity)
    }

    /// Original identity bytes; no ownership or authentication is implied.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.length)]
    }
    /// Distinguish an explicit client identifier from the hardware fallback.
    pub fn is_option_61(&self) -> bool {
        self.option_61
    }
}

impl core::fmt::Debug for ClientIdentity {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ClientIdentity")
            .field("length", &self.length)
            .field("option_61", &self.option_61)
            .finish_non_exhaustive()
    }
}

/// A lease key scoped to a trusted ingress network; DHCP identity is spoofable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientKey {
    /// Actual network selected by the packet adapter/owner, not an HTTP field.
    pub network: NetworkId,
    /// Option 61 when supplied; otherwise Ethernet identity.
    pub identity: ClientIdentity,
}
