//! IronGate operator entry point.

mod args;
mod doctor;
mod output;

use args::{HELP, Invocation, parse, requests_json};
use irongate_config::check_file;
use output::{config_json, config_text, print_stdout, write_json};
use serde_json::json;
use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

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
                write_json(&config_json(&report))?;
            } else {
                print_stdout(&config_text(&report))?;
            }
            Ok(report.exit_code)
        }
        Ok(Invocation::Doctor(options)) => {
            let report = doctor::run(&options.path, options.network.as_deref());
            if options.json {
                write_json(&doctor::json(&report))?;
            } else {
                print_stdout(&doctor::text(&report))?;
            }
            Ok(report.exit_code)
        }
        Ok(Invocation::Host { json }) => {
            let report = doctor::host();
            if json {
                write_json(&doctor::host_json(&report))?;
            } else {
                print_stdout(&doctor::host_text(&report))?;
            }
            Ok(report.exit_code())
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
