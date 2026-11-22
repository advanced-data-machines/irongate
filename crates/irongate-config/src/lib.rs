//! Offline TOML configuration validation, independent of the packet/session core.
//!
//! Configuration parsing may allocate. TOML strings are borrowed where the parser
//! permits it; Serde deserialization is disabled. No network or secret-file access
//! is performed. Passing these checks does not establish runtime readiness.

mod input;
mod report;
mod requirements;
mod schema;
mod validate;

pub use input::{check_file, check_str};
pub(crate) use report::locate_last;
pub use report::{Finding, Report, Severity, Summary};
pub use requirements::{HostRequirements, NetworkRequirement};

/// Maximum configuration size, checked before parsing.
pub const MAX_CONFIG_BYTES: usize = 1024 * 1024;
/// Maximum retained findings; excess findings set an explicit truncation flag.
pub const MAX_FINDINGS: usize = 256;
