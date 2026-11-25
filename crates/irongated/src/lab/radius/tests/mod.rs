//! Independent Python hashlib/HMAC vectors, not snapshots of the Rust codec.
use super::*;
fn config() -> config::Config {
    config::Config {
        endpoint: "192.0.2.80:1812".parse().unwrap(),
        nas_identifier: "test".into(),
        secret: zeroize::Zeroizing::new(b"test-secret-long-enough".to_vec()),
        timeout: std::time::Duration::from_secs(2),
    }
}
fn vector(name: &str) -> Vec<u8> {
    let values: serde_json::Value = serde_json::from_str(include_str!("vectors.json")).unwrap();
    let text = values[name].as_str().unwrap();
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn pap_obfuscation_and_request_authentication_match_an_independent_vector() {
    let config = config();
    let credentials = credentials::Credentials::new("lab-user", "lab-pass").unwrap();
    let mut packet = [0; 4096];
    let len = request::encode(&config, &credentials, [7; 32], &mut packet);
    assert_eq!(&packet[..len], vector("request"));
    let max = credentials::Credentials::new(&"u".repeat(64), &"p".repeat(128)).unwrap();
    assert!(request::encode(&config, &max, [9; 32], &mut packet) < 512);
}
#[test]
fn only_authenticated_supported_policy_can_accept_and_all_truncations_fail() {
    let config = config();
    let request = vector("request");
    let accept = vector("accept");
    for (name, expected) in [
        ("accept", Outcome::Accept { seconds: 5 }),
        ("reject", Outcome::Reject),
        ("challenge", Outcome::Unsupported),
        ("unsupported", Outcome::Unsupported),
        ("zero_timeout", Outcome::Unsupported),
        ("missing_ma", Outcome::Invalid),
        ("wrong_id_valid_auth", Outcome::Invalid),
        ("bad_ma_valid_response_auth", Outcome::Invalid),
        ("duplicate_timeout", Outcome::Invalid),
    ] {
        assert_eq!(
            response::decode(&vector(name), &request, &config.secret),
            expected,
            "{name}"
        );
    }
    for end in 0..accept.len() {
        assert_eq!(
            response::decode(&accept[..end], &request, &config.secret),
            Outcome::Invalid
        );
    }
    for index in 0..accept.len() {
        let mut corrupted = accept.clone();
        corrupted[index] ^= 1;
        assert_eq!(
            response::decode(&corrupted, &request, &config.secret),
            Outcome::Invalid
        );
    }
    let mut old_context = request.clone();
    old_context[4] ^= 1;
    assert_eq!(
        response::decode(&accept, &old_context, &config.secret),
        Outcome::Invalid
    );
    assert_eq!(
        response::decode(&accept, &request, b"different-secret"),
        Outcome::Invalid
    );
    assert_eq!(
        response::decode(&accept, &[], &config.secret),
        Outcome::Invalid
    );
}
