//! Compact host table and structured optional metadata.

use super::model::{Detail, Inventory};
use serde_json::{Value, json};
use std::fmt::{Display, Write as _};

pub(crate) fn json(inventory: &Inventory) -> Value {
    let cards: Vec<Value> = inventory
        .cards
        .iter()
        .map(|card| {
            json!({
                "name": card.name, "kind": card.kind,
                "admin_up": field(&card.admin_up, |v| json!(v)),
                "operstate": field(&card.operstate, |v| json!(v)),
                "carrier": field(&card.carrier, |v| json!(v)),
                "link_active": field(&card.link_active, |v| json!(v)),
                "mtu": field(&card.mtu, |v| json!(v)),
                "driver": field(&card.driver, |v| json!(v)),
                "speed_mbps": field(&card.speed_mbps, |v| json!(v)),
                "duplex": field(&card.duplex, |v| json!(v)),
                "speed_basis": "driver_reported_link_speed_not_throughput",
            })
        })
        .collect();
    let active = inventory
        .cards
        .iter()
        .filter(|card| card.link_active.value() == Some(&true))
        .count();
    let active_devices = inventory
        .cards
        .iter()
        .filter(|card| {
            matches!(card.kind, "ethernet" | "wireless" | "device")
                && card.link_active.value() == Some(&true)
        })
        .count();
    json!({
        "source": inventory.source, "observed_at_unix_seconds": inventory.observed_at_unix_seconds,
        "snapshot_atomic": false, "complete": inventory.complete(), "truncated": inventory.truncated,
        "error_kind": inventory.error_kind.map(|issue| issue.as_str()),
        "unsupported_platform": inventory.unsupported_platform,
        "discovered_interfaces": inventory.discovered_interfaces,
        "sampled_interfaces": inventory.cards.len(), "sampled_active_links": active,
        "sampled_active_device_interfaces": active_devices,
        "sampled_unknown_link_states": inventory.cards.iter().filter(|card| card.link_active.value().is_none()).count(),
        "interfaces": cards,
    })
}

fn field<T>(detail: &Detail<T>, encode: impl Fn(&T) -> Value) -> Value {
    let mut value = json!({"status": detail.status(), "value": detail.value().map(encode)});
    match detail {
        Detail::Unknown(reason) | Detail::Unsupported(reason) => value["reason"] = json!(reason),
        Detail::Unavailable(issue) => value["error_kind"] = json!(issue.as_str()),
        Detail::Reported(_) => {}
    }
    value
}

fn display<T: Display>(detail: &Detail<T>) -> String {
    match detail {
        Detail::Reported(value) => value.to_string().escape_debug().to_string(),
        _ => detail.status().to_owned(),
    }
}

pub(crate) fn text(inventory: &Inventory) -> String {
    let mut output = format!(
        "Host network inventory: {} discovered, {} sampled{}\n",
        inventory.discovered_interfaces,
        inventory.cards.len(),
        if inventory.truncated {
            " (truncated)"
        } else {
            ""
        }
    );
    if inventory.unsupported_platform {
        output.push_str("Host inventory currently supports Linux only.\n");
    }
    if let Some(issue) = inventory.error_kind {
        let _ = writeln!(output, "Inventory incomplete: {}", issue.as_str());
    }
    let _ = writeln!(
        output,
        "{:<16} {:<10} {:<7} {:<14} {:<7} {:<12} {:<12} {:<7} DRIVER",
        "INTERFACE", "KIND", "ADMIN", "STATE", "ACTIVE", "SPEED Mb/s", "DUPLEX", "MTU"
    );
    for card in &inventory.cards {
        let admin = match card.admin_up.value() {
            Some(true) => "up",
            Some(false) => "down",
            None => "unknown",
        };
        let active = match card.link_active.value() {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        let _ = writeln!(
            output,
            "{:<16} {:<10} {:<7} {:<14} {:<7} {:<12} {:<12} {:<7} {}",
            card.name.escape_debug().to_string(),
            card.kind,
            admin,
            display(&card.operstate),
            active,
            display(&card.speed_mbps),
            display(&card.duplex),
            display(&card.mtu),
            display(&card.driver)
        );
        for (name, detail) in [
            ("speed", reason(&card.speed_mbps)),
            ("duplex", reason(&card.duplex)),
        ] {
            if let Some(reason) = detail {
                let _ = writeln!(output, "  {} {name}: {reason}", card.name.escape_debug());
            }
        }
    }
    output.push_str("Active means local link evidence, not traffic delivery. Speed is driver-reported, not measured throughput.\n");
    output
}

fn reason<T>(detail: &Detail<T>) -> Option<&'static str> {
    match detail {
        Detail::Unknown(reason) | Detail::Unsupported(reason) => Some(reason),
        Detail::Unavailable(issue) => Some(issue.as_str()),
        Detail::Reported(_) => None,
    }
}

pub(crate) fn report_json(inventory: &Inventory) -> Value {
    json!({
        "schema_version": 1, "command": "doctor", "scope": "host_network",
        "result": if inventory.complete() { "reported" } else { "incomplete" },
        "complete": inventory.complete(), "runtime_verified": false,
        "exit_code": inventory.exit_code(), "config": null, "host_network": json(inventory),
    })
}

pub(crate) fn report_text(inventory: &Inventory) -> String {
    let mut output = text(inventory);
    output.push_str(
        "Read-only host inventory. No TOML loaded, packets sent or network settings changed.\n",
    );
    output
        .push_str("Addresses, routes, gateway enforcement and backend health remain unverified.\n");
    output
}
