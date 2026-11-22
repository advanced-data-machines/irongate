//! Doctor text/JSON presentation without host operations.

use super::report::{DoctorReport, Observed};
use serde_json::{Value, json};
use std::fmt::Write as _;

pub(crate) fn json(report: &DoctorReport) -> Value {
    let checks: Vec<Value> = report.checks.iter().map(|check| {
        let mut value = json!({
            "code": check.code, "subject": check.subject, "status": check.status.as_str(),
            "required": check.required, "message": check.message, "next_action": check.next_action,
        });
        if let Some(observed) = &check.observed { value["observed"] = observation(observed); }
        if let Some(expected) = &check.expected { value["expected"] = observation(expected); }
        if let Some(issue) = check.error_kind { value["error_kind"] = json!(issue.as_str()); }
        value
    }).collect();
    let interfaces: Vec<Value> = report.interfaces.iter().map(|interface| json!({
        "name": interface.name, "roles": interface.roles, "minimum_mtu": interface.minimum_mtu,
    })).collect();
    let mut result = json!({
        "schema_version": 1, "command": "doctor", "result": report.result,
        "scope": "local_interfaces", "complete": report.complete,
        "runtime_verified": false, "exit_code": report.exit_code,
        "config_path": report.config.config_path, "network": report.network,
        "observed_at_unix_seconds": report.observed_at_unix_seconds,
        "snapshot_atomic": false, "source": report.source,
        "requested_interfaces": report.requested_interfaces,
        "interfaces": interfaces, "checks": checks,
        "config": crate::output::config_json(&report.config),
    });
    if let Some(inventory) = &report.host_network {
        result["host_network"] = super::inventory::json(inventory);
    }
    result
}

fn observation(value: &Observed) -> Value {
    match value {
        Observed::Text(value) => json!(value),
        Observed::Number(value) => json!(value),
        Observed::Bool(value) => json!(value),
    }
}

fn display(value: &Observed) -> String {
    match value {
        Observed::Text(value) => value.escape_debug().to_string(),
        Observed::Number(value) => value.to_string(),
        Observed::Bool(value) => value.to_string(),
    }
}

pub(crate) fn text(report: &DoctorReport) -> String {
    let mut output = format!(
        "Doctor {} (local interface scope): {}\n",
        report.result,
        report.config.config_path.escape_debug()
    );
    if let Some(network) = &report.network {
        let _ = writeln!(output, "Network: {}", network.escape_debug());
    }
    if let Some(inventory) = &report.host_network {
        output.push_str(&super::inventory::text(inventory));
    }
    for finding in &report.config.findings {
        let _ = writeln!(
            output,
            "{} [{}] {}: {}\n  Next: {}",
            finding.severity.as_str(),
            finding.code,
            finding.field.escape_debug(),
            finding.message,
            finding.next_action
        );
    }
    for interface in &report.interfaces {
        let _ = writeln!(
            output,
            "Interface {} — {} — minimum MTU {}",
            interface.name.escape_debug(),
            interface.roles.join(", "),
            interface.minimum_mtu
        );
    }
    for check in &report.checks {
        let _ = write!(
            output,
            "{} [{}] {}",
            check.status.as_str(),
            check.code,
            check.subject.escape_debug()
        );
        if let Some(value) = &check.observed {
            let _ = write!(output, " observed={}", display(value));
        }
        if let Some(value) = &check.expected {
            let _ = write!(output, " expected={}", display(value));
        }
        if let Some(issue) = check.error_kind {
            let _ = write!(output, " reason={}", issue.as_str());
        }
        let _ = writeln!(output, ": {}", check.message);
        if check.status != super::report::Status::Pass {
            let _ = writeln!(output, "  Next: {}", check.next_action);
        }
    }
    if report.config.findings_truncated {
        output.push_str(
            "Configuration findings were truncated; correct those findings and repeat.\n",
        );
    }
    output.push_str(
        "Read-only, non-atomic sysfs observations. No packets sent or network settings changed.\n",
    );
    output.push_str("Passing this scope does not establish gateway readiness or backend health.\n");
    output
}
