//! Doctor evidence and scoped result/exit-code rules.

use irongate_config::Report;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Pass,
    Fail,
    Unverified,
    Unsupported,
    Disabled,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unverified => "unverified",
            Self::Unsupported => "unsupported",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug)]
pub(crate) enum Observed {
    Text(String),
    Number(u64),
    Bool(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadIssue {
    Missing,
    Permission,
    Invalid,
    Io,
}

impl ReadIssue {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "not_found",
            Self::Permission => "permission_denied",
            Self::Invalid => "invalid_data",
            Self::Io => "read_failed",
        }
    }
}

#[derive(Debug)]
pub(crate) struct Check {
    pub code: &'static str,
    pub subject: String,
    pub status: Status,
    pub required: bool,
    pub observed: Option<Observed>,
    pub expected: Option<Observed>,
    pub error_kind: Option<ReadIssue>,
    pub message: &'static str,
    pub next_action: &'static str,
}

impl Check {
    pub fn new(
        code: &'static str,
        subject: &str,
        status: Status,
        required: bool,
        message: &'static str,
        next_action: &'static str,
    ) -> Self {
        Self {
            code,
            subject: subject.to_owned(),
            status,
            required,
            observed: None,
            expected: None,
            error_kind: None,
            message,
            next_action,
        }
    }

    pub fn values(mut self, observed: Observed, expected: Observed) -> Self {
        self.observed = Some(observed);
        self.expected = Some(expected);
        self
    }

    pub fn unavailable(
        code: &'static str,
        subject: &str,
        required: bool,
        issue: ReadIssue,
    ) -> Self {
        let mut check = Self::new(
            code,
            subject,
            Status::Unverified,
            required,
            "Requested evidence could not be read or interpreted; this does not prove the link is down.",
            "Check sysfs availability, attribute support and read access; no changes were attempted.",
        );
        check.error_kind = Some(issue);
        check
    }
}

#[derive(Debug)]
pub(crate) struct Interface {
    pub name: String,
    pub roles: Vec<String>,
    pub minimum_mtu: u32,
}

#[derive(Debug)]
pub(crate) struct DoctorReport {
    pub config: Report,
    pub network: Option<String>,
    pub result: &'static str,
    pub complete: bool,
    pub exit_code: u8,
    pub observed_at_unix_seconds: Option<u64>,
    pub source: &'static str,
    pub interfaces: Vec<Interface>,
    pub requested_interfaces: usize,
    pub checks: Vec<Check>,
    pub host_network: Option<super::inventory::Inventory>,
}

impl DoctorReport {
    pub fn new(config: Report, network: Option<&str>, source: &'static str) -> Self {
        Self {
            config,
            network: network.map(str::to_owned),
            result: "passed",
            complete: true,
            exit_code: 0,
            observed_at_unix_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|duration| duration.as_secs()),
            source,
            interfaces: Vec::new(),
            requested_interfaces: 0,
            checks: Vec::new(),
            host_network: None,
        }
    }

    pub fn finish(mut self) -> Self {
        if self.config.exit_code != 0 || self.exit_code == 2 {
            self.result = "invalid";
            self.complete = false;
            if self.config.exit_code != 0 {
                self.exit_code = self.config.exit_code;
            }
            return self;
        }
        let required_missing = self.checks.iter().any(|check| {
            check.required && matches!(check.status, Status::Unverified | Status::Unsupported)
        });
        self.complete = !required_missing;
        if self
            .checks
            .iter()
            .any(|check| check.required && check.status == Status::Fail)
        {
            self.result = "blocked";
            self.exit_code = 1;
        } else if required_missing {
            self.result = "incomplete";
            self.exit_code = if self.checks.iter().any(|check| {
                check.required
                    && (check.status == Status::Unsupported
                        || check.error_kind == Some(ReadIssue::Permission))
            }) {
                3
            } else {
                4
            };
        }
        self
    }
}
