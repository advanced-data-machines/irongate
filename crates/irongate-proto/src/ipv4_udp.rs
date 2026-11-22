//! Strict IPv4/UDP input with checksum verification and no reassembly.

use crate::{Error, Layer, MAX_FRAME_BYTES, checksum};

/// Borrowed UDP payload and fixed-size endpoint metadata.
#[derive(Clone, Copy, Debug)]
pub struct Datagram<'a> {
    /// IPv4 source address.
    pub source: [u8; 4],
    /// IPv4 destination address.
    pub destination: [u8; 4],
    /// UDP source port.
    pub source_port: u16,
    /// UDP destination port.
    pub destination_port: u16,
    /// Payload restricted to declared IP and UDP lengths.
    pub payload: &'a [u8],
    /// IPv4 permits zero UDP checksum; false means no UDP integrity evidence.
    pub udp_checksum_present: bool,
}

/// Parse UDP in an IPv4 datagram, ignoring bytes beyond IPv4 total length.
pub fn parse(bytes: &[u8]) -> Result<Datagram<'_>, Error> {
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(Error::LimitExceeded(Layer::Ipv4));
    }
    if bytes.len() < 20 {
        return Err(Error::Truncated(Layer::Ipv4));
    }
    if bytes[0] >> 4 != 4 {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    let ihl = usize::from(bytes[0] & 15) * 4;
    if ihl < 20 {
        return Err(Error::Malformed(Layer::Ipv4));
    }
    if bytes.len() < ihl {
        return Err(Error::Truncated(Layer::Ipv4));
    }
    if ihl != 20 {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    let total = usize::from(u16::from_be_bytes([bytes[2], bytes[3]]));
    if total < 28 {
        return Err(Error::Malformed(Layer::Ipv4));
    }
    if total > bytes.len() {
        return Err(Error::Truncated(Layer::Ipv4));
    }
    let fragment = u16::from_be_bytes([bytes[6], bytes[7]]);
    if fragment & 0x8000 != 0 || bytes[8] == 0 {
        return Err(Error::Malformed(Layer::Ipv4));
    }
    if fragment & 0x3fff != 0 || bytes[9] != 17 {
        return Err(Error::Unsupported(Layer::Ipv4));
    }
    if checksum::finish(checksum::sum(&bytes[..20])) != 0 {
        return Err(Error::Checksum(Layer::Ipv4));
    }
    let source = bytes[12..16].try_into().expect("checked IPv4 length");
    let destination = bytes[16..20].try_into().expect("checked IPv4 length");
    let udp = &bytes[20..total];
    let length = usize::from(u16::from_be_bytes([udp[4], udp[5]]));
    if length < 8 || length != udp.len() {
        return Err(Error::Malformed(Layer::Udp));
    }
    let present = udp[6..8] != [0, 0];
    if present && checksum::udp(source, destination, udp) != 0 {
        return Err(Error::Checksum(Layer::Udp));
    }
    Ok(Datagram {
        source,
        destination,
        source_port: u16::from_be_bytes([udp[0], udp[1]]),
        destination_port: u16::from_be_bytes([udp[2], udp[3]]),
        payload: &udp[8..],
        udp_checksum_present: present,
    })
}
