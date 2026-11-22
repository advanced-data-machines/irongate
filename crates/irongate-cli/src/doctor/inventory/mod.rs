//! Bounded host-wide inventory; independent of TOML validation and deployment roles.

mod metrics;
mod model;
mod render;
mod sample;

use crate::doctor::{attributes, report::ReadIssue};
pub(crate) use model::Inventory;
pub(crate) use render::{json, report_json, report_text, text};
use std::collections::BTreeSet;
use std::path::Path;

const MAX_CARDS: usize = 64;

pub(crate) fn run() -> Inventory {
    if cfg!(target_os = "linux") {
        collect(Path::new("/sys/class/net"), "/sys/class/net")
    } else {
        let mut inventory = Inventory::new("unsupported_platform");
        inventory.unsupported_platform = true;
        inventory
    }
}

pub(super) fn collect(root: &Path, source: &'static str) -> Inventory {
    let mut inventory = Inventory::new(source);
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            inventory.error_kind = Some(attributes::issue(error));
            return inventory;
        }
    };
    // Keep only the lexicographically first 64 names, while counting all entries.
    // This bounds storage and makes truncated results deterministic.
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                inventory.record_error(attributes::issue(error));
                continue;
            }
        };
        inventory.discovered_interfaces += 1;
        let Ok(name) = entry.file_name().into_string() else {
            inventory.record_error(ReadIssue::Invalid);
            continue;
        };
        names.insert(name);
        if names.len() > MAX_CARDS {
            names.pop_last();
            inventory.truncated = true;
        }
    }
    inventory.cards = names
        .into_iter()
        .map(|name| sample::sample(root, name))
        .collect();
    inventory
}
