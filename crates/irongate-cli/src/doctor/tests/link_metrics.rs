//! Avoid mistaking cached or unknown link metadata for negotiated speed.

use super::Fixture;
use crate::doctor::inventory::{collect, json};
use serde_json::Value;

fn card(fixture: &Fixture) -> Value {
    json(&collect(&fixture.0, "test_fixture"))["interfaces"][0].clone()
}

#[test]
fn down_link_suppresses_stale_speed_and_duplex() {
    let fixture = Fixture::new();
    fixture.interface("eth0", 1500);
    fixture.write("eth0/speed", b"1000");
    fixture.write("eth0/duplex", b"full");
    fixture.write("eth0/carrier", b"0");
    let value = card(&fixture);
    for field in ["speed_mbps", "duplex"] {
        assert!(value[field]["value"].is_null());
        assert_eq!(value[field]["reason"], "link_not_active");
    }
}

#[test]
fn missing_carrier_does_not_become_false_or_zero_speed() {
    let fixture = Fixture::new();
    fixture.interface("eth0", 1500);
    std::fs::remove_file(fixture.0.join("eth0/carrier")).unwrap();
    let value = card(&fixture);
    assert_eq!(value["link_active"]["status"], "unknown");
    assert!(value["link_active"]["value"].is_null());
    assert_eq!(value["speed_mbps"]["reason"], "link_state_unverified");
}

#[test]
fn unknown_driver_speeds_are_not_unsigned_gigantic_values() {
    let fixture = Fixture::new();
    fixture.interface("eth0", 1500);
    for raw in ["-1", "0", "4294967295"] {
        fixture.write("eth0/speed", raw.as_bytes());
        let value = card(&fixture);
        assert_eq!(value["speed_mbps"]["status"], "unknown");
        assert!(value["speed_mbps"]["value"].is_null());
    }
}

#[test]
fn malformed_and_missing_metrics_have_distinct_statuses() {
    let fixture = Fixture::new();
    fixture.interface("eth0", 1500);
    assert_eq!(card(&fixture)["speed_mbps"]["status"], "unsupported");
    for raw in [b"fast".as_slice(), b"4294967296", &[255], &[b'1'; 257]] {
        fixture.write("eth0/speed", raw);
        let value = card(&fixture);
        assert_eq!(value["speed_mbps"]["status"], "unavailable");
        assert_eq!(value["speed_mbps"]["error_kind"], "invalid_data");
    }
    for (raw, status) in [
        ("half", "reported"),
        ("unknown", "unknown"),
        ("bad", "unavailable"),
    ] {
        fixture.write("eth0/duplex", raw.as_bytes());
        assert_eq!(card(&fixture)["duplex"]["status"], status);
    }
}

#[test]
fn unknown_operstate_with_positive_carrier_is_active_loopback() {
    let fixture = Fixture::new();
    fixture.interface("lo", 65536);
    fixture.write("lo/type", b"772");
    fixture.write("lo/operstate", b"unknown");
    let value = card(&fixture);
    assert_eq!(value["kind"], "loopback");
    assert_eq!(value["link_active"]["value"], true);
    assert_eq!(value["speed_mbps"]["status"], "unsupported");
}
