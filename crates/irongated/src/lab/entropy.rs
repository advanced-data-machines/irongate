//! Shared OS entropy for bounded control-plane workers.
use rustix::rand::{GetRandomFlags, getrandom};
use std::io;
pub(super) fn bytes() -> io::Result<[u8; 32]> {
    let mut value = [0; 32];
    let mut used = 0;
    for _ in 0..8 {
        match getrandom(&mut value[used..], GetRandomFlags::NONBLOCK) {
            Ok(0) => break,
            Ok(length) => {
                used += length;
                if used == value.len() {
                    return Ok(value);
                }
            }
            Err(rustix::io::Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::other("OS entropy unavailable"))
}
