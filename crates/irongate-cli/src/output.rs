//! Configuration report rendering and stdout writers.

use irongate_config::Report;
use serde_json::{Value, json};
use std::io::{self, Write};

pub(crate) fn config_json(report: &Report) -> Value {
    let findings: Vec<Value> = report.findings.iter().map(|finding| {
        let mut value = json!({
            "code": finding.code, "severity": finding.severity.as_str(),
            "field": finding.field, "message": finding.message, "next_action": finding.next_action,
        });
        if let Some(line) = finding.line { value["line"] = json!(line); }
        if let Some(column) = finding.column { value["column"] = json!(column); }
        value
    }).collect();
    let mut result = json!({
        "schema_version": report.schema_version, "command": report.command, "result": report.result,
        "complete": report.complete, "runtime_verified": report.runtime_verified,
        "config_path": report.config_path, "exit_code": report.exit_code,
        "findings": findings, "findings_truncated": report.findings_truncated,
    });
    if let Some(summary) = &report.summary {
        result["summary"] = json!({
            "networks": summary.networks, "max_sessions": summary.max_sessions,
            "radius_pools": summary.radius_pools, "packet_storage_bytes": summary.packet_storage_bytes,
        });
    }
    result
}

pub(crate) fn config_text(report: &Report) -> String {
    use std::fmt::Write as _;
    let mut output = format!(
        "Configuration {}: {}\n",
        report.result,
        report.config_path.escape_debug()
    );
    for finding in &report.findings {
        let _ = write!(
            output,
            "{} [{}] {}",
            finding.severity.as_str(),
            finding.code,
            finding.field.escape_debug()
        );
        if let Some(line) = finding.line {
            let _ = write!(
                output,
                " (line {line}, column {})",
                finding.column.unwrap_or(1)
            );
        }
        let _ = writeln!(
            output,
            ": {}\n  Next: {}",
            finding.message, finding.next_action
        );
    }
    if report.findings_truncated {
        output.push_str("More findings were omitted; fix these and check again.\n");
    }
    if let Some(summary) = &report.summary {
        let _ = writeln!(
            output,
            "{} network(s); {} configured session slots; {} RADIUS pool(s).",
            summary.networks, summary.max_sessions, summary.radius_pools
        );
        let _ = writeln!(
            output,
            "Packet buffer storage only: {} bytes (not a total RAM estimate).",
            summary.packet_storage_bytes
        );
    }
    output.push_str(
        "Static checks only. Host, secret files, services, and gateway behavior are unverified.\n",
    );
    output
}

pub(crate) fn write_json(value: &Value) -> io::Result<()> {
    let mut output = io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, value)?;
    writeln!(output)
}

pub(crate) fn print_stdout(text: &str) -> io::Result<()> {
    io::stdout().lock().write_all(text.as_bytes())
}
