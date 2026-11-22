//! Configuration CLI contracts. Only private temporary files are created.

mod common;
use common::{Scratch, command, report};
use std::path::Path;
use std::process::Output;

fn check(path: &Path) -> Output {
    command()
        .args(["config", "check", "--json", "--config"])
        .arg(path)
        .output()
        .unwrap()
}

#[test]
fn all_examples_pass_through_the_real_cli() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for name in ["irongate.toml", "standalone.toml", "vlan-portals.toml"] {
        let path = root.join("examples").join(name);
        let before = std::fs::read(&path).unwrap();
        let value = report(&check(&path), 0);
        assert_eq!(value["result"], "passed");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "validation must not edit its input"
        );
    }
}

#[test]
fn flags_work_on_either_side_of_the_command_and_json_is_not_colored() {
    let scratch = Scratch::new();
    let path = scratch.write(
        "profile.toml",
        include_bytes!("../../../examples/standalone.toml"),
    );
    let output = command()
        .arg("--config")
        .arg(&path)
        .args(["--format", "json", "config", "check", "--no-color"])
        .output()
        .unwrap();
    let value = report(&output, 0);
    assert_eq!(value["command"], "config.check");
    assert!(!output.stdout.contains(&0x1b));
    let output = command()
        .args(["config", "--json", "check"])
        .arg(format!("--config={}", path.display()))
        .output()
        .unwrap();
    report(&output, 0);
}

#[test]
fn default_filename_is_relative_to_the_working_directory() {
    let scratch = Scratch::new();
    scratch.write(
        "irongate.toml",
        include_bytes!("../../../examples/standalone.toml"),
    );
    let value = report(
        &command()
            .current_dir(&scratch.0)
            .args(["config", "check", "--json"])
            .output()
            .unwrap(),
        0,
    );
    assert_eq!(value["config_path"], "irongate.toml");
}

#[test]
fn missing_invalid_and_nonregular_inputs_have_structured_failures() {
    let scratch = Scratch::new();
    let missing = report(&check(&scratch.0.join("missing.toml")), 2);
    assert_eq!(missing["findings"][0]["code"], "config.input.not_found");
    assert_eq!(missing["complete"], false);
    let directory = report(&check(&scratch.0), 2);
    assert_eq!(directory["findings"][0]["code"], "config.input.not_regular");
    let path = scratch.write("bad.toml", b"schema_version = \"TOP_SECRET\"");
    let value = report(&check(&path), 2);
    assert_eq!(value["result"], "invalid");
    assert!(!value.to_string().contains("TOP_SECRET"));
    let path = scratch.write("invalid-utf8.toml", &[0xff, 0xfe]);
    assert_eq!(
        report(&check(&path), 2)["findings"][0]["code"],
        "config.input.encoding"
    );
}

#[test]
fn json_escapes_unusual_file_paths_and_unknown_keys() {
    let scratch = Scratch::new();
    let source = format!(
        "{}\n\"unusual\\\"key\" = true\n",
        include_str!("../../../examples/standalone.toml")
    );
    let path = scratch.write("quoted\"-ü.toml", source.as_bytes());
    let value = report(&check(&path), 2);
    assert_eq!(value["config_path"], path.to_string_lossy().as_ref());
    assert_eq!(value["findings"][0]["field"], "capacity.unusual\"key");
}

#[test]
fn usage_failures_remain_json_and_never_execute_unimplemented_commands() {
    for args in [
        vec!["apply", "--json"],
        vec!["config", "check", "--json", "--typo"],
        vec!["config", "check", "--json", "--config"],
        vec!["config", "check", "--config", "--json"],
        vec!["config", "check", "--json", "--format", "text"],
        vec!["config", "check", "--json", "--network", "guest"],
        vec!["doctor", "--json", "--network"],
        vec!["doctor", "--json", "--network", "../unsafe"],
        vec![
            "doctor",
            "--json",
            "--network",
            "guest",
            "--network",
            "staff",
        ],
    ] {
        let value = report(&command().args(args).output().unwrap(), 2);
        assert_eq!(value["command"], "cli");
        assert_eq!(value["findings"][0]["code"], "cli.arguments.invalid");
    }
}

#[test]
fn human_help_and_errors_are_usable_without_a_terminal() {
    let help = command().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("config check"));
    assert!(help.stderr.is_empty());
    let version = command().arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("ig "));
    let error = command().arg("--unknown").output().unwrap();
    assert_eq!(error.status.code(), Some(2));
    assert!(error.stdout.is_empty());
    assert!(String::from_utf8_lossy(&error.stderr).contains("ig --help"));
}
