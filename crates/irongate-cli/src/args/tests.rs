//! Parsing inventory requests must not require reading TOML or the host.

use super::{Invocation, parse};
use std::ffi::OsString;

fn args(values: &[&str]) -> Vec<OsString> {
    values.iter().map(OsString::from).collect()
}

#[test]
fn host_inventory_has_explicit_config_free_invocation() {
    for values in [vec!["doctor", "--host"], vec!["--host", "doctor"]] {
        assert!(matches!(
            parse(&args(&values)),
            Ok(Invocation::Host { json: false })
        ));
    }
    assert!(matches!(
        parse(&args(&["doctor", "--host", "--json"])),
        Ok(Invocation::Host { json: true })
    ));
}

#[test]
fn host_inventory_rejects_ignored_or_duplicate_options() {
    for values in [
        vec!["doctor", "--host", "--config", "ignored.toml"],
        vec!["doctor", "--host", "--network", "staff"],
        vec!["doctor", "--host", "--host"],
        vec!["config", "check", "--host"],
    ] {
        assert!(parse(&args(&values)).is_err());
    }
}
