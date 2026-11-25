//! Legacy RADIUS primitives from vetted RustCrypto implementations, not TLS.
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
pub(crate) fn digest(parts: &[&[u8]]) -> [u8; 16] {
    let mut h = Md5::new();
    for bytes in parts {
        Digest::update(&mut h, bytes);
    }
    h.finalize().into()
}
pub(super) fn mac(secret: &[u8], packet: &[u8]) -> [u8; 16] {
    let mut h = Hmac::<Md5>::new_from_slice(secret).expect("HMAC accepts any key length");
    h.update(packet);
    h.finalize().into_bytes().into()
}
pub(super) fn verify(secret: &[u8], parts: &[&[u8]], expected: &[u8]) -> bool {
    let mut h = Hmac::<Md5>::new_from_slice(secret).expect("HMAC accepts any key length");
    for bytes in parts {
        h.update(bytes);
    }
    h.verify_slice(expected).is_ok()
}
