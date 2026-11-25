//! RFC 2866 authenticators and RFC 2869 64-bit octets, without packet allocation.
use super::{config::Config, model::*};
use crate::lab::{profile, radius::crypto};
use subtle::ConstantTimeEq;
pub(super) fn encode(config: &Config, dispatch: Dispatch, id: u8, out: &mut [u8; 512]) -> usize {
    out.fill(0);
    out[0] = 4;
    out[1] = id;
    let r = dispatch.record;
    let mut at = 20;
    attr(out, &mut at, 1, r.user.bytes());
    attr(out, &mut at, 4, &profile::WAN_IP);
    attr(out, &mut at, 5, &1u32.to_be_bytes());
    attr(out, &mut at, 8, &profile::CLIENT_IP);
    attr(out, &mut at, 31, b"02:00:00:00:00:02");
    attr(out, &mut at, 32, config.radius.nas_identifier.as_bytes());
    attr(out, &mut at, 44, &r.session_id);
    let status = match r.kind {
        Kind::Start => 1,
        Kind::Stop => 2,
        Kind::Interim => 3,
    };
    for (kind, value) in [(40, status), (41, dispatch.delay), (45, 1)] {
        attr(out, &mut at, kind, &value.to_be_bytes());
    }
    if r.kind != Kind::Start {
        for (kind, value) in [
            (46, r.session_seconds.min(u64::from(u32::MAX)) as u32),
            (42, r.octets.input as u32),
            (43, r.octets.output as u32),
            (52, (r.octets.input >> 32) as u32),
            (53, (r.octets.output >> 32) as u32),
        ] {
            attr(out, &mut at, kind, &value.to_be_bytes());
        }
    }
    if r.kind == Kind::Stop {
        attr(out, &mut at, 49, &r.cause.to_be_bytes());
    }
    out[2..4].copy_from_slice(&(at as u16).to_be_bytes());
    let authenticator = crypto::digest(&[&out[..at], &config.radius.secret]);
    out[4..20].copy_from_slice(&authenticator);
    at
}
fn attr(out: &mut [u8], at: &mut usize, kind: u8, value: &[u8]) {
    out[*at] = kind;
    out[*at + 1] = (value.len() + 2) as u8;
    out[*at + 2..*at + 2 + value.len()].copy_from_slice(value);
    *at += 2 + value.len();
}
pub(super) fn acknowledged(packet: &[u8], request: &[u8], secret: &[u8]) -> bool {
    // This first interoperability profile accepts only an empty Code 5 reply.
    if packet.len() != 20
        || request.len() < 20
        || packet[0] != 5
        || packet[1] != request[1]
        || packet[2..4] != [0, 20]
    {
        return false;
    }
    let expected = crypto::digest(&[&packet[..4], &request[4..20], secret]);
    packet[4..20].ct_eq(&expected).unwrap_u8() == 1
}
