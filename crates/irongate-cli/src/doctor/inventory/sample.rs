//! One observed host interface, including physical and virtual devices.

use super::{
    metrics,
    model::{Card, Detail},
};
use crate::doctor::{attributes, report::ReadIssue};
use std::path::Path;

pub(super) fn sample(root: &Path, name: String) -> Card {
    let directory = root.join(&name);
    let admin_up = detail(attributes::admin_up(&directory));
    let carrier = detail(attributes::carrier(&directory));
    let operstate = match attributes::read(&directory.join("operstate")) {
        Ok(value)
            if matches!(
                value.as_str(),
                "up" | "unknown" | "down" | "dormant" | "testing" | "notpresent" | "lowerlayerdown"
            ) =>
        {
            Detail::Reported(value)
        }
        Ok(_) => Detail::Unavailable(ReadIssue::Invalid),
        Err(issue) => Detail::Unavailable(issue),
    };
    let link_active = active(&admin_up, &carrier, &operstate);
    let speed_mbps = metrics::speed(&directory, link_active.value());
    let duplex = metrics::duplex(&directory, link_active.value());
    let driver = match attributes::driver(&directory) {
        Ok(Some(value)) => Detail::Reported(value),
        Ok(None) => Detail::Unsupported("device_driver_link_not_exposed"),
        Err(issue) => Detail::Unavailable(issue),
    };
    Card {
        name,
        kind: kind(&directory),
        admin_up,
        operstate,
        carrier,
        link_active,
        mtu: detail(attributes::mtu(&directory)),
        driver,
        speed_mbps,
        duplex,
    }
}

fn detail<T>(value: Result<T, ReadIssue>) -> Detail<T> {
    match value {
        Ok(value) => Detail::Reported(value),
        Err(issue) => Detail::Unavailable(issue),
    }
}

fn active(
    admin: &Detail<bool>,
    carrier: &Detail<bool>,
    operstate: &Detail<String>,
) -> Detail<bool> {
    if admin.value() == Some(&false)
        || carrier.value() == Some(&false)
        || operstate
            .value()
            .is_some_and(|state| !matches!(state.as_str(), "up" | "unknown"))
    {
        return Detail::Reported(false);
    }
    if admin.value() == Some(&true) && carrier.value() == Some(&true) && operstate.value().is_some()
    {
        Detail::Reported(true)
    } else {
        Detail::Unknown("insufficient_link_evidence")
    }
}

fn kind(directory: &Path) -> &'static str {
    if !std::fs::metadata(directory).is_ok_and(|metadata| metadata.is_dir()) {
        return "unknown";
    }
    let protocol = attributes::read(&directory.join("type")).ok();
    if protocol.as_deref() == Some("772") {
        return "loopback";
    } // ARPHRD_LOOPBACK.
    if std::fs::metadata(directory.join("wireless")).is_ok_and(|metadata| metadata.is_dir()) {
        return "wireless";
    }
    match std::fs::metadata(directory.join("device")) {
        Ok(_) if protocol.as_deref() == Some("1") => "ethernet", // ARPHRD_ETHER.
        Ok(_) => "device",
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "virtual",
        Err(_) => "unknown",
    }
}
