//! Real CLI evidence distinguishes placement without echoing scoped identifiers.

mod common;
use common::{Scratch, command, report};

#[test]
fn portal_placement_is_obvious_in_text_and_json() {
    let scratch = Scratch::new();
    for (name, source, hosting, scoped, description) in [
        (
            "local",
            include_bytes!("../../../examples/standalone.toml").as_slice(),
            "local",
            false,
            "local (on this device)",
        ),
        (
            "cloud",
            include_bytes!("../../../examples/cloud-portal.toml").as_slice(),
            "external",
            true,
            "external (separate LAN/cloud service)",
        ),
    ] {
        let path = scratch.write(&format!("{name}.toml"), source);
        let output = command()
            .args(["config", "check", "--json", "--config"])
            .arg(&path)
            .output()
            .unwrap();
        let value = report(&output, 0);
        assert_eq!(value["portal_profiles"][0]["hosting"], hosting);
        assert_eq!(value["portal_profiles"][0]["tenant_scoped"], scoped);
        assert_eq!(value["portal_profiles"][0]["hosting_explicit"], true);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("tenant-a"));
        let text = command()
            .args(["config", "check", "--config"])
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(text.status.code(), Some(0));
        assert!(String::from_utf8_lossy(&text.stdout).contains(description));
    }
}
