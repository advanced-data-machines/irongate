//! Read-only doctor orchestration. Interface rules and presentation are separate.

mod attributes;
mod inventory;
mod linux;
mod render;
mod report;
mod requirements;

use irongate_config::{Report, check_file};
use report::{Check, DoctorReport, Interface, Observed, Status};
use std::path::Path;

pub(crate) use inventory::{report_json as host_json, report_text as host_text, run as host};
pub(crate) use render::{json, text};

const MAX_INTERFACES: usize = 64;

pub(crate) fn run(config_path: &Path, network: Option<&str>) -> DoctorReport {
    let config = check_file(config_path);
    let mut report = diagnose(
        config,
        network,
        if cfg!(target_os = "linux") {
            "/sys/class/net"
        } else {
            "unsupported_platform"
        },
        |interface| {
            if cfg!(target_os = "linux") {
                linux::inspect(interface, Path::new("/sys/class/net"))
            } else {
                vec![Check::new(
                    "platform.unsupported",
                    &interface.name,
                    Status::Unsupported,
                    true,
                    "This doctor backend currently supports Linux only.",
                    "Run on the intended Linux host; no platform operations were attempted.",
                )]
            }
        },
    );
    if report.config.exit_code == 0 && report.exit_code != 2 {
        report.host_network = Some(inventory::run());
    }
    report
}

fn diagnose(
    mut config: Report,
    network: Option<&str>,
    source: &'static str,
    inspect: impl Fn(&Interface) -> Vec<Check>,
) -> DoctorReport {
    let requirements = config.host_requirements.take();
    let mut report = DoctorReport::new(config, network, source);
    if report.config.exit_code != 0 {
        report.checks.push(Check::new(
            "config.validation",
            "configuration",
            Status::Fail,
            true,
            "Configuration failed validation; host inspection was skipped.",
            "Correct the findings in the configuration report first.",
        ));
        return report.finish();
    }
    let requirements = requirements.expect("valid configuration provides host requirements");
    let networks = requirements::select(&requirements, network);
    if networks.is_empty() {
        report.exit_code = 2;
        report.checks.push(Check::new(
            "doctor.network.unknown",
            "network",
            Status::Fail,
            true,
            "Requested network profile is not defined; host inspection was skipped.",
            "Choose a name from the configuration's [[networks]] entries.",
        ));
        return report.finish();
    }
    report.checks.push(Check::new(
        "config.validation",
        "configuration",
        Status::Pass,
        true,
        "Configuration passed static validation; warnings remain in its report.",
        "Review warnings before deployment.",
    ));
    let interfaces = requirements::interfaces(&requirements.upstream, &networks);
    report.requested_interfaces = interfaces.len();
    if interfaces.len() > MAX_INTERFACES {
        report.checks.push(
            Check::new(
                "doctor.interface_limit",
                "interfaces",
                Status::Unverified,
                true,
                "Interface inspection is capped at 64 distinct interfaces per request.",
                "Use --network to narrow the inspection; omitted interfaces are not verified.",
            )
            .values(
                Observed::Number(interfaces.len() as u64),
                Observed::Number(MAX_INTERFACES as u64),
            ),
        );
    }
    for interface in interfaces.into_iter().take(MAX_INTERFACES) {
        report.checks.extend(inspect(&interface));
        report.interfaces.push(interface);
    }
    for (code, enabled) in [
        (
            "radius.authentication",
            networks.iter().any(|n| n.radius_authentication),
        ),
        (
            "radius.accounting",
            networks.iter().any(|n| n.radius_accounting),
        ),
    ] {
        report.checks.push(if enabled {
            Check::new(code, "selected networks", Status::Unverified, false,
                "RADIUS is configured, but no protocol exchange or backend observation was performed.",
                "A future explicit verification test must establish backend health; doctor sends no probes.")
        } else {
            Check::new(code, "selected networks", Status::Disabled, false,
                "Selected profiles do not use this RADIUS service.", "No RADIUS backend is required for this selection.")
        });
    }
    report.checks.push(Check::new(
        "services.reachability",
        "DNS and portal",
        Status::Unverified,
        false,
        "DNS and portal connectivity were not probed.",
        "Verify from a subscriber in the isolated VM lab; no traffic was generated.",
    ));
    report.checks.push(Check::new("gateway.enforcement", "gateway", Status::Unverified, false,
        "Link checks do not prove authorization, accounting, isolation or forwarding correctness.",
        "Gateway runtime is not implemented; retain independent client verification as a release gate."));
    if networks.iter().any(|n| n.vlan_id.is_some()) {
        report.checks.push(Check::new("vlan.path", "selected VLANs", Status::Unverified, false,
            "Configured trunk bindings are shown; VLAN tags, switch settings and return paths were not verified.",
            "Test each VLAN from a subscriber; parent interface presence alone does not establish VLAN isolation."));
    }
    report.finish()
}

#[cfg(test)]
mod tests;
