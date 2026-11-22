//! A small explicit work bound, not a production deployment capacity promise.

use super::{Error, NetworkId};

/// Pilot limit per network; address and client scans are bounded by this value.
pub const MAX_LEASES: usize = 256;

/// One direct-client IPv4 pool. Configuration must be validated before use.
#[derive(Clone, Copy, Debug)]
pub struct PoolConfig {
    /// Ingress network identity.
    pub network: NetworkId,
    /// Nonzero owner-assigned arena epoch, unique across pool construction/reload.
    pub epoch: u64,
    /// Local DHCP server/gateway address.
    pub server: [u8; 4],
    /// Subnet prefix length, 1–30 in this pilot.
    pub prefix: u8,
    /// First allocatable address, inclusive.
    pub first: [u8; 4],
    /// Last allocatable address, inclusive; gateway must be outside this range.
    pub last: [u8; 4],
    /// Binding lifetime in seconds.
    pub lease_seconds: u32,
    /// Temporary offer reservation lifetime in seconds.
    pub offer_seconds: u32,
    /// Quarantine duration after a matching DECLINE.
    pub decline_seconds: u32,
}

impl PoolConfig {
    pub(super) fn validate(self) -> Result<usize, Error> {
        if self.epoch == 0
            || !(1..=30).contains(&self.prefix)
            || self.lease_seconds == 0
            || self.offer_seconds == 0
            || self.decline_seconds == 0
        {
            return Err(Error::InvalidConfig);
        }
        let first = u32::from_be_bytes(self.first);
        let last = u32::from_be_bytes(self.last);
        let server = u32::from_be_bytes(self.server);
        let size = last
            .checked_sub(first)
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::InvalidConfig)?;
        if size as u64 > MAX_LEASES as u64
            || (first..=last).contains(&server)
            || !self.host(self.server)
            || !self.host(self.first)
            || !self.host(self.last)
        {
            return Err(Error::InvalidConfig);
        }
        Ok(size as usize)
    }

    pub(super) fn host(self, ip: [u8; 4]) -> bool {
        if !(1..=30).contains(&self.prefix) || ip[0] == 0 || ip[0] == 127 || ip[0] >= 224 {
            return false;
        }
        let mask = u32::MAX << (32 - self.prefix);
        let subnet = u32::from_be_bytes(self.server) & mask;
        let address = u32::from_be_bytes(ip);
        address & mask == subnet && address != subnet && address != (subnet | !mask)
    }
}
