//! Caller-owned request packet with PAP obfuscation and mandatory HMAC-MD5.
use super::{config::Config, credentials::Credentials, crypto};
use zeroize::Zeroizing;
pub(super) fn encode(
    config: &Config,
    credentials: &Credentials,
    entropy: [u8; 32],
    out: &mut [u8; 4096],
) -> usize {
    out.fill(0);
    out[0] = 1;
    out[1] = entropy[16];
    out[4..20].copy_from_slice(&entropy[..16]);
    let mut at = 20;
    attr(out, &mut at, 1, credentials.user());
    let mut password = Zeroizing::new([0; 128]);
    let length = credentials.password().len().div_ceil(16) * 16;
    password[..credentials.password().len()].copy_from_slice(credentials.password());
    let mut previous = [0; 16];
    previous.copy_from_slice(&entropy[..16]);
    for block in password[..length].as_chunks_mut::<16>().0 {
        let hash = crypto::digest(&[&config.secret, &previous]);
        for (byte, mask) in block.iter_mut().zip(hash) {
            *byte ^= mask;
        }
        previous.copy_from_slice(block);
    }
    attr(out, &mut at, 2, &password[..length]);
    attr(out, &mut at, 4, &crate::lab::profile::WAN_IP);
    attr(out, &mut at, 5, &1u32.to_be_bytes());
    attr(out, &mut at, 6, &1u32.to_be_bytes());
    attr(out, &mut at, 8, &crate::lab::profile::CLIENT_IP);
    attr(out, &mut at, 31, b"02:00:00:00:00:02");
    attr(out, &mut at, 32, config.nas_identifier.as_bytes());
    let ma = at + 2;
    attr(out, &mut at, 80, &[0; 16]);
    out[2..4].copy_from_slice(&(at as u16).to_be_bytes());
    let mac = crypto::mac(&config.secret, &out[..at]);
    out[ma..ma + 16].copy_from_slice(&mac);
    at
}
fn attr(out: &mut [u8], at: &mut usize, kind: u8, value: &[u8]) {
    out[*at] = kind;
    out[*at + 1] = (value.len() + 2) as u8;
    out[*at + 2..*at + 2 + value.len()].copy_from_slice(value);
    *at += 2 + value.len();
}
