//! Doctor CLI checks that stop before any host inspection.

mod common;
use common::{Scratch, command, report};

#[test]
fn doctor_reports_invalid_config_and_unknown_profile_without_host_inspection() {
    let scratch = Scratch::new();
    let invalid = scratch.write("invalid.toml", b"schema_version = 1");
    let output = command()
        .args(["doctor", "--json", "--config"])
        .arg(invalid)
        .output()
        .unwrap();
    let value = report(&output, 2);
    assert_eq!(value["command"], "doctor");
    assert_eq!(value["result"], "invalid");
    assert_eq!(value["config"]["exit_code"], 2);
    assert_eq!(value["interfaces"], serde_json::json!([]));
    let valid = scratch.write(
        "valid.toml",
        include_bytes!("../../../examples/standalone.toml"),
    );
    let output = command()
        .arg("--config")
        .arg(&valid)
        .args(["--network=missing", "doctor", "--format=json"])
        .output()
        .unwrap();
    let value = report(&output, 2);
    assert_eq!(value["checks"][0]["code"], "doctor.network.unknown");
    let text = command()
        .arg("--config")
        .arg(valid)
        .args(["doctor", "--network", "missing"])
        .output()
        .unwrap();
    assert_eq!(text.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&text.stdout).contains("doctor.network.unknown"));
}
