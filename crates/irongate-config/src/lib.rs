//! Offline TOML configuration validation, independent of the packet/session core.
//!
//! Configuration parsing may allocate. TOML strings are borrowed where the parser
//! permits it; Serde deserialization is disabled. No network or secret-file access
//! is performed. Passing these checks does not establish runtime readiness.

mod schema;
mod validate;

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Maximum configuration size, checked before parsing.
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
/// Maximum retained findings; excess findings set an explicit truncation flag.
pub const MAX_FINDINGS: usize = 256;

/// Severity of a configuration finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Prevents static validation from passing.
    Error,
    /// Requires attention but does not prevent static validation.
    Warning,
}

impl Severity {
    /// Stable lowercase name for text and machine output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

/// One actionable finding. Raw source lines and supplied values are not echoed.
#[derive(Debug)]
pub struct Finding {
    /// Stable identifier for scripts.
    pub code: String,
    /// Error or warning.
    pub severity: Severity,
    /// Schema path with zero-based array indexes.
    pub field: String,
    /// Human-readable explanation.
    pub message: String,
    /// Suggested correction or verification step.
    pub next_action: String,
    /// One-based source line when available.
    pub line: Option<usize>,
    /// One-based character column when available.
    pub column: Option<usize>,
}

/// Configured limits, not measured device capacity.
#[derive(Debug)]
pub struct Summary {
    /// Number of subscriber profiles.
    pub networks: usize,
    /// Global configured session limit.
    pub max_sessions: u64,
    /// Number of configured RADIUS pools.
    pub radius_pools: usize,
    /// Packet buffer count times size; excludes all other memory overhead.
    pub packet_storage_bytes: u64,
}

/// Result of static validation for human and machine consumers.
#[derive(Debug)]
pub struct Report {
    /// Version of the output contract, independent of the input schema version.
    pub schema_version: u32,
    /// Stable operation name.
    pub command: &'static str,
    /// Either `passed` or `invalid`.
    pub result: &'static str,
    /// False if the input could not be read.
    pub complete: bool,
    /// Always false: no host or gateway readiness has been verified.
    pub runtime_verified: bool,
    /// Caller-provided file path or label.
    pub config_path: String,
    /// Exit code: 0 success, 2 invalid input, 3 read permission failure.
    pub exit_code: u8,
    /// Findings in deterministic validation order.
    pub findings: Vec<Finding>,
    /// True if the report reached its finding limit.
    pub findings_truncated: bool,
    /// Present after schema validation succeeds.
    pub summary: Option<Summary>,
}

impl Report {
    fn new(label: &str) -> Self {
        Self {
            schema_version: 1,
            command: "config.check",
            result: "passed",
            complete: true,
            runtime_verified: false,
            config_path: label.to_owned(),
            exit_code: 0,
            findings: Vec::new(),
            findings_truncated: false,
            summary: None,
        }
    }

    pub(crate) fn error(&mut self, code: &str, field: &str, message: &str, action: &str) {
        self.result = "invalid";
        self.exit_code = 2;
        self.push(Severity::Error, code, field, message, action);
    }

    pub(crate) fn warning(&mut self, code: &str, field: &str, message: &str, action: &str) {
        self.push(Severity::Warning, code, field, message, action);
    }

    fn push(&mut self, severity: Severity, code: &str, field: &str, message: &str, action: &str) {
        if self.findings.len() == MAX_FINDINGS {
            self.findings_truncated = true;
            return;
        }
        self.findings.push(Finding {
            code: code.to_owned(),
            severity,
            field: field.to_owned(),
            message: message.to_owned(),
            next_action: action.to_owned(),
            line: None,
            column: None,
        });
    }
}

/// Validate a UTF-8 TOML document in memory without side effects.
pub fn check_str(input: &str, label: &str) -> Report {
    let mut report = Report::new(label);
    if input.len() > MAX_CONFIG_BYTES {
        report.error(
            "config.input.too_large",
            "",
            "Configuration exceeds the 1 MiB input limit.",
            "Reduce the file size before validating it.",
        );
        return report;
    }
    let table = match toml::de::DeTable::parse(input) {
        Ok(table) => table,
        Err(error) => {
            report.error(
                "config.toml.invalid",
                "",
                "Invalid TOML syntax or duplicate key/table.",
                "Correct the TOML at the indicated location; source text is not echoed.",
            );
            locate_last(&mut report, input, error.span());
            return report;
        }
    };
    schema::check(table.get_ref(), input, &mut report);
    if report.exit_code == 0 {
        validate::check(table.get_ref(), &mut report);
    }
    report
}

/// Read one regular UTF-8 file, using a bounded read, then perform static checks.
///
/// Referenced interfaces, servers and secret files are not accessed. This reads
/// no more than the size limit plus one byte, and rejects nonregular inputs.
pub fn check_file(path: &Path) -> Report {
    let label = path.to_string_lossy();
    let read = || -> io::Result<Vec<u8>> {
        // Reject ordinary devices/directories/FIFOs before opening. This is not
        // a security boundary against hostile concurrent replacement of a path.
        if !std::fs::metadata(path)?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        let file = File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    match read() {
        Ok(bytes) => {
            if bytes.len() > MAX_CONFIG_BYTES {
                let mut report = Report::new(&label);
                report.error(
                    "config.input.too_large",
                    "",
                    "Configuration exceeds the 1 MiB input limit.",
                    "Reduce the file size before validating it.",
                );
                return report;
            }
            match std::str::from_utf8(&bytes) {
                Ok(input) => check_str(input, &label),
                Err(_) => {
                    let mut report = Report::new(&label);
                    report.error(
                        "config.input.encoding",
                        "",
                        "Configuration is not valid UTF-8.",
                        "Save the TOML file using UTF-8 encoding.",
                    );
                    report
                }
            }
        }
        Err(error) => {
            let mut report = Report::new(&label);
            let (code, message) = match error.kind() {
                io::ErrorKind::PermissionDenied => (
                    "config.input.permission",
                    "Permission denied reading the configuration.",
                ),
                io::ErrorKind::NotFound => (
                    "config.input.not_found",
                    "Configuration file does not exist.",
                ),
                io::ErrorKind::InvalidInput => (
                    "config.input.not_regular",
                    "Configuration must be a regular file.",
                ),
                _ => (
                    "config.input.read_failed",
                    "Could not read the configuration file.",
                ),
            };
            report.error(
                code,
                "",
                message,
                "Check --config, file type, and read permissions.",
            );
            report.complete = false;
            if error.kind() == io::ErrorKind::PermissionDenied {
                report.exit_code = 3;
            }
            report
        }
    }
}

pub(crate) fn locate_last(report: &mut Report, input: &str, span: Option<std::ops::Range<usize>>) {
    if report.findings_truncated {
        return;
    }
    if let (Some(finding), Some(span)) = (report.findings.last_mut(), span) {
        let mut offset = span.start.min(input.len());
        while !input.is_char_boundary(offset) {
            offset -= 1;
        }
        let prefix = &input[..offset];
        finding.line = Some(prefix.bytes().filter(|byte| *byte == b'\n').count() + 1);
        finding.column = Some(prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1);
    }
}
