//! Configuration gating, VLAN scope, deduplication and incomplete evidence.

use super::{Fixture, LOCAL, VLANS, check};
use crate::doctor::report::{Check, ReadIssue, Status};
use crate::doctor::{diagnose, render};
use std::cell::Cell;

#[test]
fn invalid_config_or_unknown_network_never_inspects_the_host() {
    let invalid = diagnose(
        irongate_config::check_str("bad = true", "fixture.toml"),
        None,
        "test_fixture",
        |_| panic!("invalid configuration must not inspect host state"),
    );
    assert_eq!(invalid.exit_code, 2);
    assert_eq!(invalid.result, "invalid");
    assert!(invalid.interfaces.is_empty());
    let unknown = diagnose(
        irongate_config::check_str(LOCAL, "fixture.toml"),
        Some("missing"),
        "test_fixture",
        |_| panic!("unknown network must not inspect host state"),
    );
    assert_eq!(unknown.exit_code, 2);
    assert_eq!(
        check(&unknown, "doctor.network.unknown", "network").status,
        Status::Fail
    );
}

#[test]
fn one_shared_trunk_is_read_once_and_retains_both_vlan_roles() {
    let calls = Cell::new(0);
    let report = diagnose(
        irongate_config::check_str(VLANS, "fixture.toml"),
        None,
        "test_fixture",
        |_| {
            calls.set(calls.get() + 1);
            Vec::new()
        },
    );
    assert_eq!(calls.get(), 2); // One upstream, one subscriber trunk.
    assert_eq!(report.requested_interfaces, 2);
    let trunk = report
        .interfaces
        .iter()
        .find(|interface| interface.name == "enp2s0")
        .unwrap();
    assert_eq!(
        trunk.roles,
        ["network:guest/vlan:10", "network:staff/vlan:20"]
    );
    assert_eq!(trunk.minimum_mtu, 1500);
    assert_eq!(
        check(&report, "radius.authentication", "selected networks").status,
        Status::Unverified
    );
    assert_eq!(
        check(&report, "radius.accounting", "selected networks").status,
        Status::Unverified
    );
    assert_eq!(
        check(&report, "vlan.path", "selected VLANs").status,
        Status::Unverified
    );
}

#[test]
fn profile_selection_changes_only_the_relevant_host_requirements() {
    let fixture = Fixture::healthy();
    fixture.write("enp2s0/mtu", b"1450\n");
    let source = VLANS.replacen(
        "vlan_id = 20\nsession_limit = 200\nmtu = 1500",
        "vlan_id = 20\nsession_limit = 200\nmtu = 1400",
        1,
    );
    assert_ne!(source, VLANS);
    assert_eq!(fixture.check(&source, None).exit_code, 1);
    let selected = fixture.check(&source, Some("staff"));
    assert_eq!(selected.exit_code, 0, "{selected:?}");
    let trunk = selected
        .interfaces
        .iter()
        .find(|interface| interface.name == "enp2s0")
        .unwrap();
    assert_eq!(trunk.minimum_mtu, 1400);
    assert_eq!(trunk.roles, ["network:staff/vlan:20"]);
    let json = render::json(&selected);
    assert_eq!(json["network"], "staff");
    assert_eq!(json["config"]["summary"]["networks"], 2); // Full config is still validated.
}

#[test]
fn unreadable_required_evidence_is_distinct_from_optional_metadata() {
    for (issue, exit_code) in [(ReadIssue::Permission, 3), (ReadIssue::Io, 4)] {
        let report = diagnose(
            irongate_config::check_str(LOCAL, "fixture.toml"),
            None,
            "test_fixture",
            |interface| {
                vec![Check::unavailable(
                    "interface.mtu",
                    &interface.name,
                    true,
                    issue,
                )]
            },
        );
        assert_eq!(report.exit_code, exit_code);
        assert_eq!(report.result, "incomplete");
        assert!(!report.complete);
    }
    let optional = diagnose(
        irongate_config::check_str(LOCAL, "fixture.toml"),
        None,
        "test_fixture",
        |interface| {
            vec![Check::unavailable(
                "interface.driver",
                &interface.name,
                false,
                ReadIssue::Permission,
            )]
        },
    );
    assert_eq!(optional.exit_code, 0);
    assert!(optional.complete);
}

#[test]
fn definite_failure_has_priority_over_incomplete_evidence() {
    let report = diagnose(
        irongate_config::check_str(LOCAL, "fixture.toml"),
        None,
        "test_fixture",
        |interface| {
            vec![
                Check::new(
                    "interface.exists",
                    &interface.name,
                    Status::Fail,
                    true,
                    "Missing.",
                    "Review config.",
                ),
                Check::unavailable(
                    "interface.mtu",
                    &interface.name,
                    true,
                    ReadIssue::Permission,
                ),
            ]
        },
    );
    assert_eq!(report.exit_code, 1);
    assert_eq!(report.result, "blocked");
    assert!(!report.complete);
}

#[test]
fn unsupported_platform_is_not_reported_as_missing_interfaces() {
    let report = diagnose(
        irongate_config::check_str(LOCAL, "fixture.toml"),
        None,
        "test_fixture",
        |interface| {
            vec![Check::new(
                "platform.unsupported",
                &interface.name,
                Status::Unsupported,
                true,
                "No supported backend.",
                "Use Linux.",
            )]
        },
    );
    assert_eq!(report.exit_code, 3);
    assert_eq!(report.result, "incomplete");
}

#[test]
fn interface_limit_is_explicit_and_does_not_claim_complete_coverage() {
    let start = LOCAL.find("[[networks]]").unwrap();
    let end = LOCAL.find("[capacity]").unwrap();
    let mut source = LOCAL[..start].to_owned();
    for index in 0..70 {
        source.push_str(
            &LOCAL[start..end]
                .replace("name = \"guest\"", &format!("name = \"guest{index}\""))
                .replace("enp2s0", &format!("igtest{index}"))
                .replace("192.168.50.", &format!("10.1.{index}."))
                .replace("session_limit = 200", "session_limit = 1"),
        );
    }
    source.push_str(&LOCAL[end..].replace("max_networks = 1", "max_networks = 70"));
    let calls = Cell::new(0);
    let report = diagnose(
        irongate_config::check_str(&source, "large-fixture.toml"),
        None,
        "test_fixture",
        |_| {
            calls.set(calls.get() + 1);
            Vec::new()
        },
    );
    assert_eq!(report.config.exit_code, 0, "{:?}", report.config);
    assert_eq!(report.exit_code, 4);
    assert_eq!(calls.get(), 64);
    assert_eq!(report.requested_interfaces, 71);
    assert_eq!(report.interfaces.len(), 64);
    assert!(!report.complete);
}
