//! Private filesystem fixtures; tests do not inspect host networking.

mod evidence;
mod inventory;
mod link_metrics;
mod scope;

use super::report::{Check, DoctorReport};
use super::{diagnose, linux};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const LOCAL: &str = include_str!("../../../../../examples/standalone.toml");
const VLANS: &str = include_str!("../../../../../examples/vlan-portals.toml");
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "irongate-doctor-test-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, relative: &str, value: &[u8]) {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, value).unwrap();
    }

    fn interface(&self, name: &str, mtu: u32) {
        for (attribute, value) in [
            ("flags", "0x1003\n".to_owned()),
            ("operstate", "up\n".to_owned()),
            ("carrier", "1\n".to_owned()),
            ("mtu", format!("{mtu}\n")),
        ] {
            self.write(&format!("{name}/{attribute}"), value.as_bytes());
        }
    }

    fn healthy() -> Self {
        let fixture = Self::new();
        fixture.interface("enp1s0", 1500);
        fixture.interface("enp2s0", 1500);
        fixture
    }

    fn check(&self, config: &str, network: Option<&str>) -> DoctorReport {
        inspect(config, network, &self.0)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn inspect(config: &str, network: Option<&str>, root: &Path) -> DoctorReport {
    diagnose(
        irongate_config::check_str(config, "fixture.toml"),
        network,
        "test_fixture",
        |interface| linux::inspect(interface, root),
    )
}

fn check<'a>(report: &'a DoctorReport, code: &str, subject: &str) -> &'a Check {
    report
        .checks
        .iter()
        .find(|check| check.code == code && check.subject == subject)
        .unwrap_or_else(|| panic!("missing {code}/{subject}: {report:?}"))
}
