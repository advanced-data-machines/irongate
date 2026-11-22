//! Bounded read-only sysfs access. No shell, socket or privileged operations.

use super::report::ReadIssue;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

const ATTRIBUTE_LIMIT: u64 = 256;

pub(super) fn issue(error: io::Error) -> ReadIssue {
    match error.kind() {
        io::ErrorKind::NotFound => ReadIssue::Missing,
        io::ErrorKind::PermissionDenied => ReadIssue::Permission,
        io::ErrorKind::InvalidData => ReadIssue::Invalid,
        _ => ReadIssue::Io,
    }
}

pub(super) fn presence(root: &Path, name: &str) -> Result<bool, ReadIssue> {
    if name.is_empty()
        || name.len() > 15
        || matches!(name, "." | "..")
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(ReadIssue::Invalid);
    }
    if !fs::metadata(root).map_err(issue)?.is_dir() {
        return Err(ReadIssue::Invalid);
    }
    match fs::metadata(root.join(name)) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(ReadIssue::Invalid),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(issue(error)),
    }
}

pub(super) fn read(path: &Path) -> Result<String, ReadIssue> {
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(issue)?
        .take(ATTRIBUTE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(issue)?;
    if bytes.len() as u64 > ATTRIBUTE_LIMIT {
        return Err(ReadIssue::Invalid);
    }
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| ReadIssue::Invalid)?
        .trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ReadIssue::Invalid);
    }
    Ok(value.to_owned())
}

pub(super) fn admin_up(directory: &Path) -> Result<bool, ReadIssue> {
    let flags = read(&directory.join("flags"))?;
    let value = u32::from_str_radix(flags.strip_prefix("0x").unwrap_or(&flags), 16)
        .map_err(|_| ReadIssue::Invalid)?;
    Ok(value & 1 != 0) // IFF_UP from Linux's interface flags ABI.
}

pub(super) fn carrier(directory: &Path) -> Result<bool, ReadIssue> {
    match read(&directory.join("carrier"))?.as_str() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(ReadIssue::Invalid),
    }
}

pub(super) fn mtu(directory: &Path) -> Result<u32, ReadIssue> {
    let value = read(&directory.join("mtu"))?
        .parse::<u32>()
        .map_err(|_| ReadIssue::Invalid)?;
    if value == 0 {
        return Err(ReadIssue::Invalid);
    }
    Ok(value)
}

pub(super) fn driver(directory: &Path) -> Result<Option<String>, ReadIssue> {
    match fs::read_link(directory.join("device/driver")) {
        Ok(target) => {
            let name = target
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(ReadIssue::Invalid)?;
            if name.is_empty()
                || name.len() > ATTRIBUTE_LIMIT as usize
                || name.chars().any(char::is_control)
            {
                return Err(ReadIssue::Invalid);
            }
            Ok(Some(name.to_owned()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match fs::metadata(directory.join("device")) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(issue(error)),
                Ok(_) => Err(ReadIssue::Missing),
            }
        }
        Err(error) => Err(issue(error)),
    }
}
