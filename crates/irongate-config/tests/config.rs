//! Configuration acceptance and regression tests; no host networking is accessed.

use irongate_config::{MAX_CONFIG_BYTES, MAX_FINDINGS, Severity, check_str};

const RADIUS: &str = include_str!("../../../examples/irongate.toml");
const LOCAL: &str = include_str!("../../../examples/standalone.toml");
const VLANS: &str = include_str!("../../../examples/vlan-portals.toml");

fn changed(source: &str, before: &str, after: &str) -> String {
    let result = source.replacen(before, after, 1);
    assert_ne!(
        source, result,
        "test mutation must target an existing field"
    );
    result
}

fn rejects(source: &str, before: &str, after: &str, code: &str) {
    let report = check_str(&changed(source, before, after), "test.toml");
    assert_eq!(report.exit_code, 2, "{report:?}");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.code == code && finding.severity == Severity::Error),
        "expected {code}: {report:?}"
    );
    assert!(!report.runtime_verified);
}

#[test]
fn all_example_profiles_pass_without_host_or_secret_files() {
    for (source, count, pools) in [(LOCAL, 1, 0), (RADIUS, 1, 1), (VLANS, 2, 1)] {
        let report = check_str(source, "example.toml");
        assert_eq!(report.exit_code, 0, "{report:?}");
        assert_eq!(report.result, "passed");
        assert!(report.complete);
        assert!(!report.runtime_verified);
        let summary = report.summary.expect("schema decoded");
        assert_eq!(summary.networks, count);
        assert_eq!(summary.radius_pools, pools);
        assert!(
            report
                .findings
                .iter()
                .all(|finding| finding.severity == Severity::Warning)
        );
    }
}

#[test]
fn vlan_bindings_are_unique_and_explicit() {
    rejects(
        VLANS,
        "vlan_id = 20",
        "vlan_id = 10",
        "network.binding.duplicate",
    );
    for invalid in ["0", "4095", "65535"] {
        rejects(
            VLANS,
            "vlan_id = 20",
            &format!("vlan_id = {invalid}"),
            "network.vlan.invalid",
        );
    }
    rejects(VLANS, "vlan_id = 20\n", "", "network.vlan.invalid");
    rejects(
        LOCAL,
        "tagging = \"untagged\"",
        "tagging = \"untagged\"\nvlan_id = 10",
        "network.vlan.unexpected",
    );
    rejects(
        VLANS,
        "name = \"staff\"",
        "name = \"guest\"",
        "network.duplicate_name",
    );
    // VLAN 10 may independently exist on two distinct subscriber ports.
    let separate_port = changed(
        VLANS,
        "name = \"staff\"\ninterface = \"enp2s0\"",
        "name = \"staff\"\ninterface = \"enp3s0\"",
    );
    let report = check_str(
        &changed(&separate_port, "vlan_id = 20", "vlan_id = 10"),
        "two-ports.toml",
    );
    assert_eq!(report.exit_code, 0, "{report:?}");
}

#[test]
fn address_and_lease_ownership_cannot_overlap() {
    for subnet in ["128.0.0.0/1", "126.0.0.0/7", "224.0.0.0/4"] {
        rejects(LOCAL, "192.168.50.0/24", subnet, "network.subnet.invalid");
    }
    rejects(
        VLANS,
        "10.50.20.0/24",
        "10.50.10.0/24",
        "network.subnet.overlap",
    );
    rejects(
        VLANS,
        "10.50.10.0/24",
        "10.50.0.0/16",
        "network.subnet.overlap",
    );
    rejects(
        LOCAL,
        "192.168.50.0/24",
        "192.168.50.1/24",
        "network.subnet.invalid",
    );
    rejects(
        LOCAL,
        "192.168.50.0/24",
        "192.168.50.0/31",
        "network.subnet.invalid",
    );
    rejects(
        LOCAL,
        "gateway = \"192.168.50.1\"",
        "gateway = \"192.168.50.255\"",
        "network.gateway.invalid",
    );
    rejects(
        LOCAL,
        "range_start = \"192.168.50.20\"",
        "range_start = \"192.168.50.1\"",
        "dhcp.range.includes_gateway",
    );
    rejects(
        LOCAL,
        "range_start = \"192.168.50.20\"",
        "range_start = \"192.168.50.251\"",
        "dhcp.range.reversed",
    );
    rejects(
        LOCAL,
        "range_end = \"192.168.50.250\"",
        "range_end = \"192.168.51.250\"",
        "dhcp.range.invalid",
    );
    rejects(
        LOCAL,
        "session_limit = 200",
        "session_limit = 232",
        "capacity.leases.insufficient",
    );
}

#[test]
fn providers_cannot_silently_fall_back_or_ignore_settings() {
    rejects(
        RADIUS,
        "radius_pool = \"central\"",
        "radius_pool = \"missing\"",
        "radius.pool.unresolved",
    );
    rejects(
        LOCAL,
        "provider = \"consent\"",
        "provider = \"local_users\"",
        "config.value.unsupported",
    );
    rejects(
        LOCAL,
        "provider = \"consent\"",
        "provider = \"consent\"\nradius_pool = \"unused\"",
        "config.value.inapplicable",
    );
    rejects(
        RADIUS,
        "radius_unavailable = \"deny_new_sessions\"",
        "radius_unavailable = \"allow_everyone\"",
        "config.value.unsupported",
    );
    rejects(
        RADIUS,
        "radius_unavailable = \"deny_new_sessions\"\n",
        "",
        "config.value.required",
    );
    rejects(
        RADIUS,
        "sink = \"radius\"",
        "sink = \"local\"",
        "network.profile.unsupported",
    );
    rejects(
        LOCAL,
        "journal_max_bytes = 67108864",
        "journal_max_bytes = 0",
        "accounting.journal.capacity",
    );
    rejects(
        RADIUS,
        "interim_seconds = 60",
        "interim_seconds = 10",
        "accounting.interval.invalid",
    );
    rejects(
        RADIUS,
        "name = \"secondary\"",
        "name = \"primary\"",
        "radius.server.duplicate_name",
    );
    rejects(
        RADIUS,
        "address = \"192.0.2.11\"",
        "address = \"192.0.2.10\"",
        "radius.server.duplicate_endpoint",
    );
    rejects(
        RADIUS,
        "transport = \"udp\"",
        "transport = \"tls\"",
        "config.value.unsupported",
    );
    rejects(
        RADIUS,
        "auth_port = 1812",
        "auth_port = 0",
        "config.value.zero",
    );
    rejects(
        RADIUS,
        "auth_port = 1812",
        "auth_port = 65536",
        "config.schema.integer_range",
    );
}

#[test]
fn capacity_must_cover_profiles_frames_and_control_headroom() {
    rejects(
        VLANS,
        "max_networks = 2",
        "max_networks = 1",
        "capacity.networks.exceeded",
    );
    rejects(
        VLANS,
        "max_sessions = 400",
        "max_sessions = 399",
        "capacity.sessions.exceeded",
    );
    rejects(
        VLANS,
        "packet_buffer_bytes = 2048",
        "packet_buffer_bytes = 1517",
        "capacity.packet_buffer.too_small",
    );
    let report = check_str(
        &changed(
            VLANS,
            "packet_buffer_bytes = 2048",
            "packet_buffer_bytes = 1518",
        ),
        "exact-frame.toml",
    );
    assert_eq!(report.exit_code, 0, "{report:?}");
    rejects(
        LOCAL,
        "shaping_queue_packets = 256",
        "shaping_queue_packets = 512",
        "capacity.shaping.exhausts_buffers",
    );
    rejects(
        RADIUS,
        "pending_radius_requests = 128",
        "pending_radius_requests = 0",
        "capacity.radius.zero",
    );
    rejects(
        LOCAL,
        "max_sessions = 200",
        "max_sessions = -1",
        "config.schema.integer_range",
    );
    rejects(
        LOCAL,
        "packet_buffers = 512",
        "packet_buffers = 4294967296",
        "config.schema.integer_range",
    );
}

#[test]
fn mode_specific_portal_fields_and_bindings_are_validated() {
    for url in [
        "/login",
        "file:///etc/passwd",
        "https://user:secret@example.test/login",
        "https://example.test:70000/login",
        "https://example.test/login#fragment",
    ] {
        rejects(
            RADIUS,
            "https://portal.example.test/login",
            url,
            "portal.url.invalid",
        );
    }
    rejects(
        LOCAL,
        "listen = \"192.168.50.1:3990\"",
        "listen = \"0.0.0.0:3990\"",
        "portal.listen.invalid",
    );
    rejects(
        LOCAL,
        "listen = \"192.168.50.1:3990\"",
        "listen = \"192.168.50.1:0\"",
        "portal.listen.invalid",
    );
    rejects(
        LOCAL,
        "terms_file = \"/etc/irongate/terms.html\"",
        "terms_file = \"relative.html\"",
        "config.path.invalid",
    );
    rejects(
        LOCAL,
        "mode = \"builtin_consent\"",
        "mode = \"builtin_consent\"\nurl = \"https://example.test\"",
        "config.value.inapplicable",
    );
    rejects(
        LOCAL,
        "interface = \"enp2s0\"",
        "interface = \"enp1s0\"",
        "interface.role_conflict",
    );
}

#[test]
fn schema_rejects_typos_and_does_not_echo_supplied_secrets() {
    let source = changed(
        RADIUS,
        "secret_file = \"/etc/irongate/secrets/portal\"",
        "shared_secret = \"DO_NOT_ECHO_THIS_SECRET\"",
    );
    let report = check_str(&source, "redaction.toml");
    assert_eq!(report.exit_code, 2);
    let finding = report
        .findings
        .iter()
        .find(|f| f.code == "config.schema.unknown_field")
        .unwrap();
    assert_eq!(finding.field, "networks[0].portal.shared_secret");
    assert!(finding.line.is_some());
    assert!(finding.column.is_some());
    assert!(!format!("{report:?}").contains("DO_NOT_ECHO_THIS_SECRET"));
    rejects(
        RADIUS,
        "max_sessions = 200",
        "max_sessions = \"DO_NOT_ECHO_THIS_SECRET\"",
        "config.schema.invalid_type",
    );
    let report = check_str(
        &changed(
            RADIUS,
            "max_sessions = 200",
            "max_sessions = \"DO_NOT_ECHO_THIS_SECRET\"",
        ),
        "types.toml",
    );
    assert!(!format!("{report:?}").contains("DO_NOT_ECHO_THIS_SECRET"));
    rejects(
        LOCAL,
        "idle_timeout_seconds = 300\n",
        "",
        "config.schema.missing_field",
    );
    rejects(
        LOCAL,
        "schema_version = 1",
        "schema_version = 2",
        "config.schema.unsupported_version",
    );
}

#[test]
fn malformed_duplicate_and_deep_inputs_report_errors_without_source_text() {
    for source in [
        "password = \"TOP_SECRET",
        "value = 1\nvalue = 2",
        "[table\nvalue = 1",
    ] {
        let report = check_str(source, "broken.toml");
        assert_eq!(report.findings[0].code, "config.toml.invalid");
        assert!(report.findings[0].line.is_some());
        assert!(!format!("{report:?}").contains("TOP_SECRET"));
    }
    let nested = format!("value = {}0{}", "[".repeat(100), "]".repeat(100));
    assert_ne!(check_str(&nested, "deep.toml").exit_code, 0);
}

#[test]
fn toml_numeric_forms_have_the_same_capacity_semantics() {
    for value in ["0x200", "0o1000", "0b1000000000", "+512", "5_12"] {
        let report = check_str(
            &changed(
                LOCAL,
                "packet_buffers = 512",
                &format!("packet_buffers = {value}"),
            ),
            "numeric.toml",
        );
        assert_eq!(report.exit_code, 0, "{value}: {report:?}");
    }
}

#[test]
fn inputs_and_findings_have_explicit_bounds() {
    let report = check_str(&" ".repeat(MAX_CONFIG_BYTES + 1), "large.toml");
    assert_eq!(report.findings[0].code, "config.input.too_large");
    let mut source = LOCAL.to_owned();
    for i in 0..MAX_FINDINGS + 20 {
        source.push_str(&format!("unknown_{i} = true\n"));
    }
    let report = check_str(&source, "many-errors.toml");
    assert_eq!(report.exit_code, 2);
    assert_eq!(report.findings.len(), MAX_FINDINGS);
    assert!(report.findings_truncated);
}
