//! Rules for passive local link evidence, separate from CLI presentation.

use super::attributes;
use super::report::{Check, Interface, Observed, ReadIssue, Status};
use std::path::Path;

pub(super) fn inspect(interface: &Interface, root: &Path) -> Vec<Check> {
    let name = &interface.name;
    let exists = match attributes::presence(root, name) {
        Ok(true) => Check::new("interface.exists", name, Status::Pass, true,
            "Configured interface exists in this sysfs view.", "No action required for this check.")
            .values(Observed::Bool(true), Observed::Bool(true)),
        Ok(false) => return vec![Check::new("interface.exists", name, Status::Fail, true,
            "Configured interface is absent from this sysfs view.",
            "Correct the configured name or inspect the intended namespace; no interface was created.")
            .values(Observed::Bool(false), Observed::Bool(true))],
        Err(issue) => return vec![Check::unavailable("interface.exists", name, true, issue)],
    };
    let directory = root.join(name);
    let mut checks = vec![exists];
    checks.push(boolean(
        "interface.admin_up",
        name,
        attributes::admin_up(&directory),
        "Interface is administratively down.",
        "Review interface ownership and a future setup plan; doctor does not bring links up.",
    ));
    checks.push(operstate(
        name,
        attributes::read(&directory.join("operstate")),
    ));
    checks.push(boolean(
        "interface.carrier",
        name,
        attributes::carrier(&directory),
        "Interface reports no carrier.",
        "Inspect the cable, switch, parent link or virtual connection; no settings were changed.",
    ));
    checks.push(match attributes::mtu(&directory) {
        Ok(mtu) => Check::new("interface.mtu", name,
            if mtu >= interface.minimum_mtu { Status::Pass } else { Status::Fail }, true,
            "Compare the current local MTU with the selected profiles' minimum; this does not verify path MTU.",
            "If too small, review the configured MTU and host change plan before applying anything.")
            .values(Observed::Number(mtu.into()), Observed::Number(interface.minimum_mtu.into())),
        Err(issue) => Check::unavailable("interface.mtu", name, true, issue),
    });
    checks.push(match attributes::driver(&directory) {
        Ok(Some(driver)) => {
            let mut check = Check::new(
                "interface.driver",
                name,
                Status::Pass,
                false,
                "Driver name was reported by the device link; capabilities were not queried.",
                "No action required for this check.",
            );
            check.observed = Some(Observed::Text(driver));
            check
        }
        Ok(None) => Check::new(
            "interface.driver",
            name,
            Status::Unsupported,
            false,
            "No device driver link is exposed; this is common for virtual interfaces.",
            "No correction is implied; hardware capabilities remain unverified.",
        ),
        Err(issue) => Check::unavailable("interface.driver", name, false, issue),
    });
    checks
}

fn boolean(
    code: &'static str,
    name: &str,
    value: Result<bool, ReadIssue>,
    fail: &'static str,
    action: &'static str,
) -> Check {
    match value {
        Ok(up) => Check::new(
            code,
            name,
            if up { Status::Pass } else { Status::Fail },
            true,
            if up {
                "Interface attribute meets this local requirement."
            } else {
                fail
            },
            if up {
                "No action required for this check."
            } else {
                action
            },
        )
        .values(Observed::Bool(up), Observed::Bool(true)),
        Err(issue) => Check::unavailable(code, name, true, issue),
    }
}

fn operstate(name: &str, value: Result<String, ReadIssue>) -> Check {
    match value {
        Ok(value) => {
            let (status, required, message) = match value.as_str() {
                "up" => (
                    Status::Pass,
                    true,
                    "Interface reports operational up; end-to-end connectivity is not established.",
                ),
                "unknown" => (
                    Status::Unverified,
                    false,
                    "Driver leaves operational state unknown; this is not proof of failure and is common on virtual links.",
                ),
                "down" | "lowerlayerdown" | "dormant" | "testing" | "notpresent" => (
                    Status::Fail,
                    true,
                    "Reported operational state does not currently permit normal user traffic.",
                ),
                _ => {
                    return Check::unavailable(
                        "interface.operstate",
                        name,
                        true,
                        ReadIssue::Invalid,
                    );
                }
            };
            Check::new("interface.operstate", name, status, required, message,
                "Use the other link evidence and an independent client test; do not infer a repair from this field alone.")
                .values(Observed::Text(value), Observed::Text("up (or driver-reported unknown)".to_owned()))
        }
        Err(issue) => Check::unavailable("interface.operstate", name, true, issue),
    }
}
