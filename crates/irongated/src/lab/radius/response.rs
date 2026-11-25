//! Authenticate before interpreting supported policy or reporting backend health.
use super::{Outcome, crypto};
use subtle::ConstantTimeEq;
pub(super) fn decode(packet: &[u8], request: &[u8], secret: &[u8]) -> Outcome {
    if request.len() < 20
        || !(20..=4096).contains(&packet.len())
        || packet.len() != usize::from(u16::from_be_bytes([packet[2], packet[3]]))
        || packet[1] != request[1]
        || !matches!(packet[0], 2 | 3 | 11)
    {
        return Outcome::Invalid;
    }
    let mut ma = None;
    let mut seconds = None;
    let mut unsupported = false;
    let mut at = 20;
    let mut count = 0;
    while at < packet.len() {
        count += 1;
        if count > 64 || at + 2 > packet.len() {
            return Outcome::Invalid;
        }
        let len = usize::from(packet[at + 1]);
        if len < 2 || at + len > packet.len() {
            return Outcome::Invalid;
        }
        match packet[at] {
            80 if len == 18 && ma.is_none() => ma = Some(at + 2),
            80 => return Outcome::Invalid,
            27 if len == 6 && seconds.is_none() => {
                seconds = Some(u32::from_be_bytes(
                    packet[at + 2..at + 6].try_into().unwrap(),
                ))
            }
            27 => return Outcome::Invalid,
            18 => (), // Reply text is never logged or reflected to the browser.
            _ => unsupported = true,
        }
        at += len;
    }
    let Some(ma) = ma else {
        return Outcome::Invalid;
    };
    let expected = crypto::digest(&[&packet[..4], &request[4..20], &packet[20..], secret]);
    if packet[4..20].ct_eq(&expected).unwrap_u8() != 1 {
        return Outcome::Invalid;
    }
    // Stream the authenticated views with the request nonce and zero MA field;
    // no full-packet copy or heap allocation is needed to validate the response.
    if !crypto::verify(
        secret,
        &[
            &packet[..4],
            &request[4..20],
            &packet[20..ma],
            &[0; 16],
            &packet[ma + 16..],
        ],
        &packet[ma..ma + 16],
    ) {
        return Outcome::Invalid;
    }
    match packet[0] {
        3 => Outcome::Reject,
        2 if !unsupported && seconds != Some(0) => Outcome::Accept {
            seconds: seconds.unwrap_or(30).min(30),
        },
        _ => Outcome::Unsupported,
    }
}
