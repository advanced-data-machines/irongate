//! Small errors without formatting or allocation on the packet path.

/// Packet layer identifying a rejected input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// Ethernet or VLAN header.
    Ethernet,
    /// Direct Ethernet/IPv4 ARP.
    Arp,
    /// IPv4 header or datagram.
    Ipv4,
    /// UDP header or checksum.
    Udp,
    /// DHCP header or options.
    Dhcp,
}

/// Bounded protocol failure. Unsupported forms are not necessarily malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Input exceeds the pilot's explicit work/byte bounds.
    LimitExceeded(Layer),
    /// Input ends before its declared structure.
    Truncated(Layer),
    /// A field or length is invalid for the supported form.
    Malformed(Layer),
    /// A valid protocol feature is outside this pilot.
    Unsupported(Layer),
    /// Checksum verification failed.
    Checksum(Layer),
    /// Caller-owned output is too small; no bytes were written.
    Capacity {
        /// Exact required output size.
        required: usize,
    },
    /// This request cannot be answered by the pilot OFFER encoder.
    NotDiscover,
    /// OFFER parameters contain an invalid address or zero lease duration.
    InvalidOffer,
    /// ACK/NAK requires a consistent direct-client DHCPREQUEST.
    NotRequest,
    /// Reply identity, requested address or lease parameters do not match.
    InvalidReply,
}
