//! IronGate's offline operator CLI. Networking commands are not implemented yet.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use irongate_config::{Report, check_file};
use serde_json::{Value, json};

const HELP: &str = "IronGate operator CLI

Usage: ig [OPTIONS] config check

Commands:
  config check           Validate a TOML file without accessing host networking

Options (accepted before or after the command):
  --config PATH          Configuration file (default: irongate.toml)
  --format text|json     Readable text or versioned JSON (default: text)
  --json                 Shorthand for --format json
  --no-color             Explicit plain output (color is never enabled yet)
  -h, --help             Show this help
  -V, --version          Show the CLI version

This checks static configuration only. Doctor, apply, and networking are not
implemented. No sockets are opened, no secret files are read, and no host network
settings are changed. See docs/configuration.md for the supported schema.
";

struct Options {
    path: PathBuf,
    json: bool,
}

enum Invocation {
    Check(Options),
    Help,
    Version,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => {
            let _ = writeln!(io::stderr(), "ig: could not write the command result");
            ExitCode::from(7)
        }
    }
}

fn run() -> io::Result<u8> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match parse(&args) {
        Ok(Invocation::Help) => {
            print_stdout(HELP)?;
            Ok(0)
        }
        Ok(Invocation::Version) => {
            print_stdout(&format!("ig {}\n", env!("CARGO_PKG_VERSION")))?;
            Ok(0)
        }
        Ok(Invocation::Check(options)) => {
            let report = check_file(&options.path);
            if options.json {
                write_json(&report_json(&report))?;
            } else {
                print_stdout(&report_text(&report))?;
            }
            Ok(report.exit_code)
        }
        Err(message) => {
            if requests_json(&args) {
                write_json(&json!({
                    "schema_version": 1, "command": "cli", "result": "invalid",
                    "complete": false, "runtime_verified": false, "exit_code": 2,
                    "findings_truncated": false,
                    "findings": [{ "code": "cli.arguments.invalid", "severity": "error",
                        "field": "arguments", "message": message,
                        "next_action": "Run ig --help for supported commands and options." }]
                }))?;
            } else {
                writeln!(io::stderr(), "ig: {message}\nRun ig --help for usage.")?;
            }
            Ok(2)
        }
    }
}

fn parse(args: &[OsString]) -> Result<Invocation, &'static str> {
    let mut path = None;
    let mut format = None;
    let mut command = Vec::new();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = arg.to_str();
        match value {
            Some("--help" | "-h") => return Ok(Invocation::Help),
            Some("--version" | "-V") => return Ok(Invocation::Version),
            Some("--no-color") => {}
            Some("--json") => set_format(&mut format, "json")?,
            Some("--config") => {
                index += 1;
                let next = args.get(index).ok_or("--config requires a file path.")?;
                if next.is_empty() || next.to_str().is_some_and(|s| s.starts_with('-')) {
                    return Err(
                        "--config requires a path; use --config=PATH for names beginning with a hyphen.",
                    );
                }
                if path.replace(PathBuf::from(next)).is_some() {
                    return Err("Specify --config only once.");
                }
            }
            Some("--format") => {
                index += 1;
                let next = args
                    .get(index)
                    .and_then(|s| s.to_str())
                    .ok_or("--format requires text or json.")?;
                set_format(&mut format, next)?;
            }
            Some(s) if s.starts_with("--config=") => {
                let next = &s[9..];
                if next.is_empty() {
                    return Err("--config requires a nonempty path.");
                }
                if path.replace(PathBuf::from(next)).is_some() {
                    return Err("Specify --config only once.");
                }
            }
            Some(s) if s.starts_with("--format=") => set_format(&mut format, &s[9..])?,
            Some(s) if s.starts_with('-') => return Err("Unknown option."),
            Some(s) => command.push(s),
            None => return Err("Command names and option names must be UTF-8."),
        }
        index += 1;
    }
    if command.is_empty() && args.is_empty() {
        return Ok(Invocation::Help);
    }
    if command != ["config", "check"] {
        return Err("Expected the implemented command: config check.");
    }
    Ok(Invocation::Check(Options {
        path: path.unwrap_or_else(|| PathBuf::from("irongate.toml")),
        json: format == Some("json"),
    }))
}

fn set_format<'a>(target: &mut Option<&'a str>, value: &'a str) -> Result<(), &'static str> {
    if !matches!(value, "text" | "json") {
        return Err("--format must be text or json.");
    }
    if target.replace(value).is_some() {
        return Err("Choose --format or --json only once.");
    }
    Ok(())
}

fn requests_json(args: &[OsString]) -> bool {
    // Preserve structured failures even when another argument is invalid. Avoid
    // treating the value of --config as an output-format request.
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        match arg.to_str() {
            Some("--config")
                if args
                    .get(index + 1)
                    .is_some_and(|s| !s.to_string_lossy().starts_with('-')) =>
            {
                index += 1
            }
            Some("--json" | "--format=json") => return true,
            Some("--format") if args.get(index + 1).is_some_and(|s| s == "json") => return true,
            _ => {}
        }
        index += 1;
    }
    false
}

fn report_json(report: &Report) -> Value {
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

fn report_text(report: &Report) -> String {
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

fn write_json(value: &Value) -> io::Result<()> {
    let mut output = io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, value)?;
    writeln!(output)
}

fn print_stdout(text: &str) -> io::Result<()> {
    io::stdout().lock().write_all(text.as_bytes())
}
