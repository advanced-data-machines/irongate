//! Small fixed command vocabulary and global option parsing.

use std::ffi::OsString;
use std::path::PathBuf;

pub(super) const HELP: &str = "IronGate operator CLI

Usage: ig [OPTIONS] <COMMAND>

Commands:
  config check           Validate a TOML file without accessing host networking
  doctor                 Read-only Linux interface preflight

Options (accepted before or after the command):
  --config PATH          Configuration file (default: irongate.toml)
  --network NAME         Inspect one subscriber profile (doctor only)
  --host                 Inventory host interfaces without TOML (doctor only)
  --format text|json     Readable text or versioned JSON (default: text)
  --json                 Shorthand for --format json
  --no-color             Explicit plain output (color is never enabled yet)
  -h, --help             Show this help
  -V, --version          Show the CLI version

Doctor validates configuration and reads Linux interface state from sysfs.
No sockets are opened, no secret files are read, and no host network settings
are changed. Gateway enforcement and backend health remain unverified.
See docs/configuration.md and docs/doctor.md for scope and exit codes.
";

pub(super) struct Options {
    pub path: PathBuf,
    pub json: bool,
    pub network: Option<String>,
}

pub(super) enum Invocation {
    Check(Options),
    Doctor(Options),
    Host { json: bool },
    Help,
    Version,
}

pub(super) fn parse(args: &[OsString]) -> Result<Invocation, &'static str> {
    let mut path = None;
    let mut format = None;
    let mut network = None;
    let mut host = false;
    let mut command = Vec::new();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        let value = arg.to_str();
        match value {
            Some("--help" | "-h") => return Ok(Invocation::Help),
            Some("--version" | "-V") => return Ok(Invocation::Version),
            Some("--no-color") => {}
            Some("--json") => set_format(&mut format, "json")?,
            Some("--host") => {
                if host {
                    return Err("Specify --host only once.");
                }
                host = true;
            }
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
            Some("--network") => {
                index += 1;
                let value = args
                    .get(index)
                    .and_then(|s| s.to_str())
                    .ok_or("--network requires a profile name.")?;
                set_network(&mut network, value)?;
            }
            Some(s) if s.starts_with("--network=") => set_network(&mut network, &s[10..])?,
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
    if host {
        if command != ["doctor"] {
            return Err("--host is supported by doctor only.");
        }
        if path.is_some() || network.is_some() {
            return Err(
                "--host does not use --config or --network; choose inventory or configured preflight.",
            );
        }
        return Ok(Invocation::Host {
            json: format == Some("json"),
        });
    }
    let options = Options {
        path: path.unwrap_or_else(|| PathBuf::from("irongate.toml")),
        json: format == Some("json"),
        network,
    };
    if command == ["doctor"] {
        return Ok(Invocation::Doctor(options));
    }
    if command != ["config", "check"] {
        return Err("Expected config check or doctor.");
    }
    if options.network.is_some() {
        return Err("--network is supported by doctor only.");
    }
    Ok(Invocation::Check(options))
}

fn set_network(target: &mut Option<String>, value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(
            "--network requires a profile name of 1–64 letters, digits, hyphens or underscores.",
        );
    }
    if target.replace(value.to_owned()).is_some() {
        return Err("Specify --network only once.");
    }
    Ok(())
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

pub(super) fn requests_json(args: &[OsString]) -> bool {
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

#[cfg(test)]
mod tests;
