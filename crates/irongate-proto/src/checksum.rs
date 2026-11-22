//! Network-order one's-complement checksums, including odd-length payloads.

pub(crate) fn sum(bytes: &[u8]) -> u32 {
    let (chunks, remainder) = bytes.as_chunks::<2>();
    let mut sum = chunks.iter().fold(0u32, |sum, pair| {
        sum + u32::from(u16::from_be_bytes([pair[0], pair[1]]))
    });
    if let [last] = remainder {
        sum += u32::from(*last) << 8;
    }
    sum
}

pub(crate) fn finish(mut sum: u32) -> u16 {
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

pub(crate) fn udp(source: [u8; 4], destination: [u8; 4], datagram: &[u8]) -> u16 {
    finish(sum(&source) + sum(&destination) + 17 + datagram.len() as u32 + sum(datagram))
}

#[cfg(test)]
mod tests {
    use super::{finish, sum};

    #[test]
    fn published_checksum_vector_and_odd_byte_padding() {
        assert_eq!(
            finish(sum(&[0, 1, 0xf2, 3, 0xf4, 0xf5, 0xf6, 0xf7])),
            0x220d
        );
        assert_eq!(finish(sum(&[1, 2, 3])), 0xfbfd);
    }
}
