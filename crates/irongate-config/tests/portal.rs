//! Hosting, tenant scope and redacted evidence; no referenced files/services read.

use irongate_config::{Severity, check_str};

const LOCAL: &str = include_str!("../../../examples/standalone.toml");
const EXTERNAL: &str = include_str!("../../../examples/irongate.toml");
const CLOUD: &str = include_str!("../../../examples/cloud-portal.toml");

fn changed(source: &str, before: &str, after: &str) -> String {
    assert!(source.contains(before));
    source.replacen(before, after, 1)
}

fn rejects(source: &str, code: &str) {
    let report = check_str(source, "fixture.toml");
    assert_eq!(report.exit_code, 2, "{report:?}");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == code && finding.severity == Severity::Error),
        "{report:?}"
    );
    assert!(report.portal_profiles.is_empty());
}

#[test]
fn hosting_is_distinct_from_mode_and_cloud_scope_is_redacted() {
    for (source, hosting, scoped) in [
        (LOCAL, "local", false),
        (EXTERNAL, "external", false),
        (CLOUD, "external", true),
    ] {
        let report = check_str(source, "fixture.toml");
        assert_eq!(report.exit_code, 0, "{report:?}");
        let portal = &report.portal_profiles[0];
        assert_eq!(portal.hosting, hosting);
        assert!(portal.hosting_explicit);
        assert_eq!(portal.tenant_scoped, scoped);
        assert!(!format!("{portal:?}").contains("tenant-a"));
        assert!(!report.runtime_verified);
    }
}

#[test]
fn mismatched_unknown_or_local_tenant_hosting_is_rejected() {
    rejects(
        &changed(LOCAL, "hosting = \"local\"", "hosting = \"external\""),
        "portal.hosting.conflict",
    );
    rejects(
        &changed(EXTERNAL, "hosting = \"external\"", "hosting = \"local\""),
        "portal.hosting.conflict",
    );
    rejects(
        &changed(EXTERNAL, "hosting = \"external\"", "hosting = \"cloud\""),
        "config.value.unsupported",
    );
    rejects(
        &changed(
            LOCAL,
            "mode = \"builtin_consent\"",
            "mode = \"builtin_consent\"\ntenant_id = \"tenant-a\"",
        ),
        "config.value.inapplicable",
    );
}

#[test]
fn tenant_scope_is_complete_bounded_and_requires_https() {
    for key in ["tenant_id", "site_id", "gateway_id"] {
        let line = CLOUD
            .lines()
            .find(|line| line.starts_with(&format!("{key} =")))
            .unwrap();
        rejects(
            &CLOUD.replace(&format!("{line}\n"), ""),
            "portal.scope.incomplete",
        );
        rejects(
            &changed(CLOUD, line, &format!("{key} = \"{}\"", "x".repeat(65))),
            "config.name.invalid",
        );
    }
    rejects(
        &CLOUD.replace(
            "https://portal.example.test/",
            "http://portal.example.test/",
        ),
        "portal.scope.https_required",
    );
}

#[test]
fn legacy_schema_one_remains_valid_with_explicit_inference_warning() {
    for source in [LOCAL, EXTERNAL] {
        let line = source
            .lines()
            .find(|line| line.starts_with("hosting ="))
            .unwrap();
        let report = check_str(&source.replace(&format!("{line}\n"), ""), "legacy.toml");
        assert_eq!(report.exit_code, 0, "{report:?}");
        assert!(!report.portal_profiles[0].hosting_explicit);
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "portal.hosting.implicit"
                    && finding.severity == Severity::Warning)
        );
    }
}

#[test]
fn vlan_profiles_can_mix_local_and_external_portals() {
    let vlans = include_str!("../../../examples/vlan-portals.toml");
    let start = vlans.find("[networks.portal]").unwrap();
    let end = vlans.find("[networks.policy]").unwrap();
    let local_start = LOCAL.find("[networks.portal]").unwrap();
    let local_end = LOCAL.find("[networks.policy]").unwrap();
    let replacement = LOCAL[local_start..local_end].replace("192.168.50.1", "10.50.10.1");
    let mut mixed = format!("{}{}{}", &vlans[..start], replacement, &vlans[end..]);
    for line in [
        "radius_unavailable = \"deny_new_sessions\"\n",
        "existing_sessions_on_radius_outage = \"retain_granted_policy\"\n",
    ] {
        mixed = mixed.replacen(line, "", 1);
    }
    let report = check_str(&mixed, "mixed.toml");
    assert_eq!(report.exit_code, 0, "{report:?}");
    assert_eq!(report.portal_profiles[0].network, "guest");
    assert_eq!(report.portal_profiles[0].hosting, "local");
    assert_eq!(report.portal_profiles[1].network, "staff");
    assert_eq!(report.portal_profiles[1].hosting, "external");
}
