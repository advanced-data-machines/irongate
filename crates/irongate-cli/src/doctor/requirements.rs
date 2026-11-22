//! Group shared subscriber trunks once, retaining their profile/VLAN roles.

use super::report::Interface;
use irongate_config::{HostRequirements, NetworkRequirement};
use std::collections::BTreeMap;

pub(super) fn select<'a>(
    requirements: &'a HostRequirements,
    network: Option<&str>,
) -> Vec<&'a NetworkRequirement> {
    requirements
        .networks
        .iter()
        .filter(|profile| network.is_none_or(|name| profile.name == name))
        .collect()
}

pub(super) fn interfaces(upstream: &str, networks: &[&NetworkRequirement]) -> Vec<Interface> {
    let mut targets = BTreeMap::new();
    let minimum_mtu = networks
        .iter()
        .map(|network| network.mtu)
        .max()
        .unwrap_or(0);
    targets.insert(
        upstream.to_owned(),
        Interface {
            name: upstream.to_owned(),
            roles: vec!["upstream".to_owned()],
            minimum_mtu,
        },
    );
    for network in networks {
        let tag = network
            .vlan_id
            .map(|id| format!("vlan:{id}"))
            .unwrap_or_else(|| "untagged".to_owned());
        let target = targets
            .entry(network.interface.clone())
            .or_insert_with(|| Interface {
                name: network.interface.clone(),
                roles: Vec::new(),
                minimum_mtu: 0,
            });
        target.minimum_mtu = target.minimum_mtu.max(network.mtu);
        target.roles.push(format!("network:{}/{tag}", network.name));
    }
    targets.into_values().collect()
}
