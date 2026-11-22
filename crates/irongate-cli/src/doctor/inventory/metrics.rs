//! Interpret driver speed/duplex conservatively; do not expose cached down-link data.

use super::model::Detail;
use crate::doctor::{attributes, report::ReadIssue};
use std::path::Path;

pub(super) fn speed(directory: &Path, active: Option<&bool>) -> Detail<u32> {
    if let Some(reason) = inactive_reason(active) {
        return Detail::Unknown(reason);
    }
    match attributes::read(&directory.join("speed")) {
        Ok(raw) if matches!(raw.as_str(), "-1" | "0" | "4294967295") => {
            Detail::Unknown("driver_reports_unknown")
        }
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 && value < u32::MAX => Detail::Reported(value),
            _ => Detail::Unavailable(ReadIssue::Invalid),
        },
        Err(ReadIssue::Missing) => Detail::Unsupported("attribute_not_exposed"),
        Err(issue) => Detail::Unavailable(issue),
    }
}

pub(super) fn duplex(directory: &Path, active: Option<&bool>) -> Detail<String> {
    if let Some(reason) = inactive_reason(active) {
        return Detail::Unknown(reason);
    }
    match attributes::read(&directory.join("duplex")) {
        Ok(raw) if matches!(raw.as_str(), "full" | "half") => Detail::Reported(raw),
        Ok(raw) if raw == "unknown" => Detail::Unknown("driver_reports_unknown"),
        Ok(_) => Detail::Unavailable(ReadIssue::Invalid),
        Err(ReadIssue::Missing) => Detail::Unsupported("attribute_not_exposed"),
        Err(issue) => Detail::Unavailable(issue),
    }
}

fn inactive_reason(active: Option<&bool>) -> Option<&'static str> {
    match active {
        Some(true) => None,
        Some(false) => Some("link_not_active"),
        None => Some("link_state_unverified"),
    }
}
