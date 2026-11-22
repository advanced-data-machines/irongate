//! Offline allocator/latency experiment; no I/O or allocation inside replay.

#[path = "../tests/common/mod.rs"]
mod common;

use irongate_proto::{dhcp::encode_offer, parse_request};
use std::hint::black_box;

fn main() {
    let iterations: u32 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "100000".to_owned())
        .parse()
        .expect("iteration count must fit u32");
    let frame = common::decode(common::TAGGED);
    let settings = common::offer();
    let mut output = [0u8; 1024];
    // Identical setup/warmup in both the zero-iteration and replay processes.
    let request = parse_request(&frame).unwrap();
    black_box(encode_offer(&request, &settings, &mut output).unwrap());
    for _ in 0..iterations {
        let request = parse_request(black_box(&frame)).unwrap();
        let length = encode_offer(&request, black_box(&settings), black_box(&mut output)).unwrap();
        black_box(&output[..length]);
    }
}
