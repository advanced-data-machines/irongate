//! Shared static configuration findings and report data.

use crate::{HostRequirements, MAX_FINDINGS};

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
    /// Available only for a valid configuration; not serialized in config reports.
    pub host_requirements: Option<HostRequirements>,
    /// Redacted portal placement for each network, only after validation passes.
    pub portal_profiles: Vec<crate::PortalProfile>,
}

impl Report {
    pub(crate) fn new(label: &str) -> Self {
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
            host_requirements: None,
            portal_profiles: Vec::new(),
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
