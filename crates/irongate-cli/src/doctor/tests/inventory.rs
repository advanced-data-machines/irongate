//! Host-wide coverage stays independent of deployment requirements.

use super::Fixture;
use crate::doctor::inventory::{collect, json, report_json, text};

#[test]
fn inventory_includes_unconfigured_devices_in_sorted_order() {
    let fixture = Fixture::healthy();
    fixture.interface("unused", 9000);
    fixture.write("unused/flags", b"0x0");
    fixture.write("enp1s0/type", b"1");
    fixture.write("enp1s0/device/marker", b"");
    fixture.write("enp1s0/speed", b"2500\n");
    fixture.write("enp1s0/duplex", b"full\n");
    fixture.write("enp2s0/wireless/marker", b"");
    let inventory = collect(&fixture.0, "test_fixture");
    let value = json(&inventory);
    assert_eq!(value["discovered_interfaces"], 3);
    assert_eq!(value["sampled_active_links"], 2);
    assert_eq!(value["sampled_active_device_interfaces"], 2);
    let cards = value["interfaces"].as_array().unwrap();
    assert_eq!(cards[0]["name"], "enp1s0");
    assert_eq!(cards[0]["kind"], "ethernet");
    assert_eq!(cards[0]["speed_mbps"]["value"], 2500);
    assert_eq!(cards[0]["duplex"]["value"], "full");
    assert_eq!(cards[1]["kind"], "wireless");
    assert_eq!(cards[2]["name"], "unused");
    assert_eq!(cards[2]["link_active"]["value"], false);
    assert_eq!(inventory.exit_code(), 0);
    assert_eq!(report_json(&inventory)["result"], "reported");
    assert!(report_json(&inventory)["config"].is_null());
    assert!(text(&inventory).contains("2500"));
}

#[test]
fn inventory_retains_first_64_names_and_reports_truncation() {
    let fixture = Fixture::new();
    for index in (0..70).rev() {
        fixture.interface(&format!("eth{index:02}"), 1500);
    }
    let inventory = collect(&fixture.0, "test_fixture");
    let value = json(&inventory);
    assert_eq!(value["discovered_interfaces"], 70);
    assert_eq!(value["sampled_interfaces"], 64);
    assert_eq!(value["interfaces"][0]["name"], "eth00");
    assert_eq!(value["interfaces"][63]["name"], "eth63");
    assert_eq!(value["truncated"], true);
    assert_eq!(value["complete"], false);
    assert_eq!(inventory.exit_code(), 4);
}

#[test]
fn missing_inventory_is_incomplete_instead_of_empty_success() {
    let fixture = Fixture::new();
    let inventory = collect(&fixture.0.join("missing"), "test_fixture");
    assert_eq!(inventory.exit_code(), 4);
    let value = report_json(&inventory);
    assert_eq!(value["result"], "incomplete");
    assert_eq!(value["host_network"]["error_kind"], "not_found");
}

#[test]
fn configured_report_can_include_independent_host_inventory() {
    let fixture = Fixture::healthy();
    fixture.interface("unused", 1500);
    fixture.write("unused/carrier", b"0");
    let mut report = fixture.check(super::LOCAL, None);
    report.host_network = Some(collect(&fixture.0, "test_fixture"));
    let value = crate::doctor::json(&report);
    assert_eq!(value["exit_code"], 0);
    assert_eq!(value["host_network"]["sampled_interfaces"], 3);
    assert_eq!(value["host_network"]["sampled_active_links"], 2);
}
