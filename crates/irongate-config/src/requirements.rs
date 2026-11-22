//! Minimal host-check requirements from already validated configuration.

use crate::schema;

/// A validated subscriber profile's minimal requirements for passive host checks.
#[derive(Debug)]
pub struct NetworkRequirement {
    /// Unique profile name.
    pub name: String,
    /// Subscriber interface, already checked against the interface-name schema.
    pub interface: String,
    /// Minimum local interface MTU for the initial forwarding profile.
    pub mtu: u32,
    /// Tag on the subscriber trunk; absent for explicit untagged traffic.
    pub vlan_id: Option<u16>,
    /// Whether this profile selects RADIUS authentication.
    pub radius_authentication: bool,
    /// Whether this profile selects RADIUS accounting.
    pub radius_accounting: bool,
}

/// Host-check inputs extracted from the same successfully validated TOML read.
/// No passwords, secret-file references, or portal credentials are included.
#[derive(Debug)]
pub struct HostRequirements {
    /// Configured upstream interface.
    pub upstream: String,
    /// Subscriber profiles and their local link requirements.
    pub networks: Vec<NetworkRequirement>,
}

pub(crate) fn extract(root: &toml::de::DeTable<'_>) -> HostRequirements {
    HostRequirements {
        upstream: schema::text(schema::child(root, "interfaces"), "upstream").to_owned(),
        networks: schema::tables(root, "networks")
            .iter()
            .map(|network| NetworkRequirement {
                name: schema::text(network, "name").to_owned(),
                interface: schema::text(network, "interface").to_owned(),
                mtu: schema::number(network, "mtu") as u32,
                vlan_id: schema::optional_number(network, "vlan_id").map(|n| n as u16),
                radius_authentication: schema::text(
                    schema::child(network, "authentication"),
                    "provider",
                ) == "radius",
                radius_accounting: schema::text(schema::child(network, "accounting"), "sink")
                    == "radius",
            })
            .collect(),
    }
}
