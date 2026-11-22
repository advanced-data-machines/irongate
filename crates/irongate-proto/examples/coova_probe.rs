//! Offline stdin/stdout adapter for the separately built Coova comparison tools.

#[path = "../tests/common/mod.rs"]
mod common;

use irongate_proto::{dhcp, parse_request};
use std::hint::black_box;
use std::io::{self, Read};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 4, "mode, option code, iterations");
    let code: u8 = args[2].parse().expect("option code");
    let iterations: u32 = args[3].parse().expect("iteration count");
    assert!((1..=100_000_000).contains(&iterations));
    let mut input = Vec::new();
    io::stdin().take(4097).read_to_end(&mut input).unwrap();
    assert!(input.len() <= 4096);
    match args[1].as_str() {
        "option" => match dhcp::parse(&input) {
            Ok(request) => {
                // Match the C search benchmark: validation happens before timing.
                let mut value = None;
                black_box(request.option(code));
                let start = Instant::now();
                for _ in 0..iterations {
                    value = black_box(request).option(black_box(code));
                    black_box(value);
                }
                let elapsed = start.elapsed().as_nanos();
                println!(
                    "{{\"status\":\"{}\",\"elapsed_ns\":{elapsed},\"value\":\"{}\"}}",
                    if value.is_some() { "found" } else { "missing" },
                    hex(value.unwrap_or(&[]))
                );
            }
            Err(error) => rejected(error),
        },
        "offer" => match parse_request(&input) {
            Ok(request) => {
                let settings = common::offer();
                let mut output = [0u8; 2048];
                let mut result = Ok(0);
                black_box(dhcp::encode_offer(&request, &settings, &mut output).unwrap_or(0));
                let start = Instant::now();
                for _ in 0..iterations {
                    result = dhcp::encode_offer(
                        black_box(&request),
                        black_box(&settings),
                        black_box(&mut output),
                    );
                    black_box(&output);
                }
                let elapsed = start.elapsed().as_nanos();
                match result {
                    Ok(length) => println!(
                        "{{\"status\":\"encoded\",\"elapsed_ns\":{elapsed},\"frame\":\"{}\"}}",
                        hex(&output[..length])
                    ),
                    Err(error) => rejected(error),
                }
            }
            Err(error) => rejected(error),
        },
        "frame" => match parse_request(&input) {
            Ok(request) => println!(
                "{{\"status\":\"accepted\",\"message_type\":{}}}",
                request.dhcp().message_type() as u8
            ),
            Err(error) => rejected(error),
        },
        _ => panic!("unsupported probe mode"),
    }
}

fn rejected(error: irongate_proto::Error) {
    // Error Debug contains only fixed enum names, punctuation and integers.
    println!("{{\"status\":\"rejected\",\"error\":\"{error:?}\",\"elapsed_ns\":0}}");
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}
