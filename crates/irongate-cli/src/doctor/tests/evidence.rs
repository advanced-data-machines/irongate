//! Distinguish absent, down, unsupported and unavailable link evidence.

use super::{Fixture, LOCAL, check};
use crate::doctor::report::{Observed, ReadIssue, Status};
use crate::doctor::{attributes, render};

#[test]
fn healthy_links_pass_only_the_declared_scope_without_editing_fixtures() {
    let fixture = Fixture::healthy();
    let before = std::fs::read(fixture.0.join("enp2s0/mtu")).unwrap();
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 0, "{report:?}");
    assert_eq!(report.result, "passed");
    assert!(report.complete);
    assert_eq!(std::fs::read(fixture.0.join("enp2s0/mtu")).unwrap(), before);
    assert_eq!(
        check(&report, "interface.driver", "enp2s0").status,
        Status::Unsupported
    );
    assert_eq!(
        check(&report, "radius.authentication", "selected networks").status,
        Status::Disabled
    );
    assert_eq!(
        check(&report, "gateway.enforcement", "gateway").status,
        Status::Unverified
    );
    let json = render::json(&report);
    assert_eq!(json["runtime_verified"], false);
    assert_eq!(json["scope"], "local_interfaces");
    assert_eq!(json["source"], "test_fixture");
    assert_eq!(json["snapshot_atomic"], false);
}

#[test]
fn missing_interface_is_a_definite_failure_but_missing_sysfs_is_not() {
    let fixture = Fixture::new();
    fixture.interface("enp1s0", 1500);
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 1);
    assert_eq!(report.result, "blocked");
    assert!(report.complete);
    assert_eq!(
        check(&report, "interface.exists", "enp2s0").status,
        Status::Fail
    );
    let report = super::inspect(LOCAL, None, &fixture.0.join("unmounted"));
    assert_eq!(report.exit_code, 4);
    assert_eq!(report.result, "incomplete");
    assert!(!report.complete);
    assert_eq!(
        check(&report, "interface.exists", "enp2s0").status,
        Status::Unverified
    );
}

#[test]
fn administrative_down_and_no_carrier_are_separate_findings() {
    let fixture = Fixture::healthy();
    fixture.write("enp2s0/flags", b"0x1002\n");
    fixture.write("enp2s0/carrier", b"0\n");
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 1);
    assert_eq!(
        check(&report, "interface.admin_up", "enp2s0").status,
        Status::Fail
    );
    assert_eq!(
        check(&report, "interface.carrier", "enp2s0").status,
        Status::Fail
    );
}

#[test]
fn dormant_and_lower_layer_down_are_blocking_operational_states() {
    let fixture = Fixture::healthy();
    for state in ["down", "lowerlayerdown", "dormant", "testing", "notpresent"] {
        fixture.write("enp2s0/operstate", state.as_bytes());
        let report = fixture.check(LOCAL, None);
        assert_eq!(report.exit_code, 1, "{state}: {report:?}");
        assert_eq!(
            check(&report, "interface.operstate", "enp2s0").status,
            Status::Fail
        );
    }
}

#[test]
fn driver_unknown_operstate_is_not_falsely_reported_as_down() {
    let fixture = Fixture::healthy();
    fixture.write("enp2s0/operstate", b"unknown\n");
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 0);
    let check = check(&report, "interface.operstate", "enp2s0");
    assert_eq!(check.status, Status::Unverified);
    assert!(!check.required);
}

#[test]
fn missing_or_malformed_attributes_are_incomplete_not_link_failure() {
    let fixture = Fixture::healthy();
    std::fs::remove_file(fixture.0.join("enp2s0/carrier")).unwrap();
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 4);
    assert_eq!(
        check(&report, "interface.carrier", "enp2s0").error_kind,
        Some(ReadIssue::Missing)
    );
    fixture.write("enp2s0/carrier", b"2\n");
    fixture.write("enp2s0/mtu", b"not-a-number\n");
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 4);
    assert_eq!(
        check(&report, "interface.mtu", "enp2s0").error_kind,
        Some(ReadIssue::Invalid)
    );
}

#[test]
fn insufficient_mtu_blocks_but_a_larger_mtu_is_compatible() {
    let fixture = Fixture::healthy();
    fixture.write("enp2s0/mtu", b"1400\n");
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 1);
    let finding = check(&report, "interface.mtu", "enp2s0");
    assert!(matches!(finding.observed, Some(Observed::Number(1400))));
    assert!(matches!(finding.expected, Some(Observed::Number(1500))));
    fixture.write("enp2s0/mtu", b"9000\n");
    assert_eq!(fixture.check(LOCAL, None).exit_code, 0);
}

#[cfg(unix)]
#[test]
fn driver_name_is_read_from_the_link_without_opening_its_target() {
    let fixture = Fixture::healthy();
    std::fs::create_dir(fixture.0.join("enp2s0/device")).unwrap();
    std::os::unix::fs::symlink(
        "/nonexistent/drivers/testnic",
        fixture.0.join("enp2s0/device/driver"),
    )
    .unwrap();
    let report = fixture.check(LOCAL, None);
    let finding = check(&report, "interface.driver", "enp2s0");
    assert_eq!(finding.status, Status::Pass);
    assert!(matches!(&finding.observed, Some(Observed::Text(name)) if name == "testnic"));
    assert_eq!(report.exit_code, 0);
}

#[test]
fn sysfs_reads_reject_oversized_or_non_utf8_values() {
    let fixture = Fixture::healthy();
    fixture.write("enp2s0/operstate", &vec![b'x'; 257]);
    fixture.write("enp2s0/mtu", &[0xff, 0xfe]);
    let report = fixture.check(LOCAL, None);
    assert_eq!(report.exit_code, 4);
    assert_eq!(
        check(&report, "interface.operstate", "enp2s0").error_kind,
        Some(ReadIssue::Invalid)
    );
    assert_eq!(
        check(&report, "interface.mtu", "enp2s0").error_kind,
        Some(ReadIssue::Invalid)
    );
    assert_eq!(
        attributes::presence(&fixture.0, "../outside"),
        Err(ReadIssue::Invalid)
    );
}
