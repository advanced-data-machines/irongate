//! Bounded configuration input, parsing and validation orchestration.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use crate::{MAX_CONFIG_BYTES, Report, locate_last, schema, validate};

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
        if report.exit_code == 0 {
            report.host_requirements = Some(crate::requirements::extract(table.get_ref()));
            report.portal_profiles = crate::portal::profiles(table.get_ref());
        }
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
