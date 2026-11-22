//! Cross-field validation after strict schema/type checks.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use ipnet::Ipv4Net;
use toml::de::DeTable;

use crate::portal::check as portal;
use crate::schema::{child, join, number, optional_number, optional_text, tables, text};
use crate::{Report, Summary};

pub(crate) fn check(root: &DeTable<'_>, report: &mut Report) {
    if number(root, "schema_version") != 1 {
        report.error(
            "config.schema.unsupported_version",
            "schema_version",
            "Only schema version 1 is supported.",
            "Use version 1 and migrate settings explicitly before changing schema versions.",
        );
    }
    let gateway = child(root, "gateway");
    name(report, "gateway.name", text(gateway, "name"));
    choice(
        report,
        "gateway.platform_mode",
        text(gateway, "platform_mode"),
        &["managed", "preconfigured"],
    );
    choice(
        report,
        "gateway.forwarding",
        text(gateway, "forwarding"),
        &["userspace"],
    );
    choice(
        report,
        "gateway.unmatched_subscriber_traffic",
        text(gateway, "unmatched_subscriber_traffic"),
        &["drop"],
    );
    let upstream = text(child(root, "interfaces"), "upstream");
    interface(report, "interfaces.upstream", upstream);

    let pools = root
        .get("radius")
        .map(|_| tables(child(root, "radius"), "pools"))
        .unwrap_or_default();
    let mut pool_names = BTreeSet::new();
    for (i, pool) in pools.iter().enumerate() {
        let path = format!("radius.pools[{i}]");
        let pool_name = text(pool, "name");
        name(report, &join(&path, "name"), pool_name);
        if !pool_names.insert(pool_name) {
            report.error(
                "radius.pool.duplicate_name",
                &join(&path, "name"),
                "RADIUS pool name is duplicated.",
                "Give each pool a unique name and update its references.",
            );
        }
        radius_pool(pool, &path, report);
    }

    let networks = tables(root, "networks");
    let capacity = child(root, "capacity");
    let mut names = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    let mut subnets = Vec::new();
    let mut used_pools = BTreeSet::new();
    let mut total_sessions = 0_u64;
    let mut required_buffer = 0_u64;
    for (i, network) in networks.iter().enumerate() {
        let path = format!("networks[{i}]");
        let network_name = text(network, "name");
        name(report, &join(&path, "name"), network_name);
        if !names.insert(network_name) {
            report.error(
                "network.duplicate_name",
                &join(&path, "name"),
                "Network name is duplicated.",
                "Use a unique name for each subscriber profile.",
            );
        }
        let device = text(network, "interface");
        interface(report, &join(&path, "interface"), device);
        if device == upstream {
            report.error(
                "interface.role_conflict",
                &join(&path, "interface"),
                "Subscriber and upstream interfaces are the same.",
                "Use separate interface bindings for the initial userspace profile.",
            );
        }
        let tagging = text(network, "tagging");
        choice(
            report,
            &join(&path, "tagging"),
            tagging,
            &["untagged", "8021q"],
        );
        let vlan = optional_number(network, "vlan_id");
        match (tagging, vlan) {
            ("8021q", Some(1..=4094)) | ("untagged", None) => {
                if !bindings.insert((device, vlan)) {
                    report.error("network.binding.duplicate", &path, "Subscriber interface/VLAN binding is duplicated.",
                        "Bind each interface/VLAN pair, including untagged traffic, to exactly one profile.");
                }
            }
            ("8021q", _) => report.error(
                "network.vlan.invalid",
                &join(&path, "vlan_id"),
                "A tagged profile requires a VLAN ID from 1 through 4094.",
                "Set an ordinary VLAN ID; priority and reserved tags are unsupported.",
            ),
            ("untagged", Some(_)) => report.error(
                "network.vlan.unexpected",
                &join(&path, "vlan_id"),
                "An untagged profile cannot specify a VLAN ID.",
                "Remove vlan_id or explicitly select 8021q tagging.",
            ),
            _ => {}
        }
        let session_limit = number(network, "session_limit");
        positive(report, &join(&path, "session_limit"), session_limit);
        // Each count is u32 and the input is bounded, so this cannot overflow u64.
        total_sessions += session_limit;
        let mtu = number(network, "mtu");
        if !(68..=65535).contains(&mtu) {
            report.error(
                "network.mtu.invalid",
                &join(&path, "mtu"),
                "IPv4 MTU must be between 68 and 65535 bytes.",
                "Choose the actual path MTU and verify it with the later host preflight.",
            );
        }
        required_buffer = required_buffer.max(mtu + if tagging == "8021q" { 18 } else { 14 });
        choice(
            report,
            &join(&path, "ipv6"),
            text(network, "ipv6"),
            &["blocked"],
        );
        let gateway = ipv4(network, &path, i, &mut subnets, report);
        portal(
            child(network, "portal"),
            &join(&path, "portal"),
            gateway,
            report,
        );
        let auth = child(network, "authentication");
        let accounting = child(network, "accounting");
        let provider = text(auth, "provider");
        let sink = text(accounting, "sink");
        choice(
            report,
            &join(&path, "authentication.provider"),
            provider,
            &["consent", "radius"],
        );
        choice(
            report,
            &join(&path, "accounting.sink"),
            sink,
            &["local", "radius"],
        );
        for (service, table, kind) in [
            ("authentication", auth, provider),
            ("accounting", accounting, sink),
        ] {
            let field = join(&path, &format!("{service}.radius_pool"));
            if kind == "radius" {
                match optional_text(table, "radius_pool") {
                    Some(pool) if pool_names.contains(pool) => {
                        used_pools.insert(pool);
                    }
                    _ => report.error(
                        "radius.pool.unresolved",
                        &field,
                        "A configured RADIUS pool reference is required.",
                        "Reference a name declared in [[radius.pools]].",
                    ),
                }
            } else {
                unused(table, "radius_pool", &join(&path, service), report);
            }
        }
        let portal_mode = text(child(network, "portal"), "mode");
        if !matches!(
            (provider, sink, portal_mode),
            ("consent", "local", "builtin_consent") | ("radius", "radius", "external_uam")
        ) {
            report.error(
                "network.profile.unsupported",
                &path,
                "Portal/authentication/accounting combination is unsupported in version 1.",
                "Use builtin_consent + consent + local, or external_uam + radius + radius.",
            );
        }
        accounting_config(accounting, &join(&path, "accounting"), report);
        let policy = child(network, "policy");
        for key in ["session_timeout_seconds", "idle_timeout_seconds"] {
            positive(
                report,
                &join(&path, &format!("policy.{key}")),
                number(policy, key),
            );
        }
        if provider == "radius" {
            required_choice(
                policy,
                "radius_unavailable",
                &join(&path, "policy"),
                &["deny_new_sessions"],
                report,
            );
            required_choice(
                policy,
                "existing_sessions_on_radius_outage",
                &join(&path, "policy"),
                &["retain_granted_policy"],
                report,
            );
        } else {
            for key in ["radius_unavailable", "existing_sessions_on_radius_outage"] {
                unused(policy, key, &join(&path, "policy"), report);
            }
        }
    }
    // Sorting avoids an all-pairs overlap scan as the number of VLANs grows.
    subnets.sort_unstable();
    let mut prior: Option<(u32, u32, usize)> = None;
    for current in subnets {
        if let Some(previous) = prior {
            if current.0 <= previous.1 {
                report.error("network.subnet.overlap", &format!("networks[{}].ipv4.subnet", current.2),
                    "Subscriber subnet overlaps another profile.",
                    &format!("Choose a subnet disjoint from networks[{}]; isolated overlapping subnets are unsupported.", previous.2));
            }
            if current.1 > previous.1 {
                prior = Some(current);
            }
        } else {
            prior = Some(current);
        }
    }
    for (i, pool) in pools.iter().enumerate() {
        if !used_pools.contains(text(pool, "name")) {
            report.warning(
                "radius.pool.unused",
                &format!("radius.pools[{i}]"),
                "RADIUS pool is not referenced by a network.",
                "Remove the unused pool or explicitly reference it from a profile.",
            );
        }
    }
    capacities(
        capacity,
        networks.len(),
        total_sessions,
        required_buffer,
        !used_pools.is_empty(),
        report,
    );
    report.summary = Some(Summary {
        networks: networks.len(),
        max_sessions: number(capacity, "max_sessions"),
        radius_pools: pools.len(),
        packet_storage_bytes: number(capacity, "packet_buffers")
            * number(capacity, "packet_buffer_bytes"),
    });
}

fn ipv4(
    network: &DeTable<'_>,
    path: &str,
    index: usize,
    subnets: &mut Vec<(u32, u32, usize)>,
    report: &mut Report,
) -> Option<Ipv4Addr> {
    let config = child(network, "ipv4");
    let subnet = text(config, "subnet").parse::<Ipv4Net>().ok();
    let usable = subnet.filter(|net| {
        (1..=30).contains(&net.prefix_len())
            && net.addr() == net.network()
            && net.network().octets()[0] > 0
            && net.broadcast().octets()[0] < 224
            && !net.contains(&Ipv4Addr::LOCALHOST)
    });
    if usable.is_none() {
        report.error(
            "network.subnet.invalid",
            &join(path, "ipv4.subnet"),
            "Expected a canonical unicast IPv4 subnet with prefix 1 through 30.",
            "Use a network address such as 192.168.50.0/24, with host bits cleared.",
        );
    }
    if let Some(net) = usable {
        subnets.push((net.network().into(), net.broadcast().into(), index));
    }
    let parse_host = |key: &str, report: &mut Report| {
        let address = text(config, key).parse::<Ipv4Addr>().ok();
        if address.is_none() || usable.is_some_and(|net| !address.is_some_and(|ip| host(net, ip))) {
            report.error(
                "network.gateway.invalid",
                &join(path, &format!("ipv4.{key}")),
                "Gateway must be a usable host address inside the subnet.",
                "Choose an IPv4 host address, excluding the subnet and broadcast addresses.",
            );
        }
        address
    };
    let gateway = parse_host("gateway", report);
    let dhcp = child(network, "dhcp");
    choice(
        report,
        &join(path, "dhcp.mode"),
        text(dhcp, "mode"),
        &["local"],
    );
    positive(
        report,
        &join(path, "dhcp.lease_seconds"),
        number(dhcp, "lease_seconds"),
    );
    let start = text(dhcp, "range_start").parse::<Ipv4Addr>().ok();
    let end = text(dhcp, "range_end").parse::<Ipv4Addr>().ok();
    for (key, value) in [("range_start", start), ("range_end", end)] {
        if value.is_none() || usable.is_some_and(|net| !value.is_some_and(|ip| host(net, ip))) {
            report.error("dhcp.range.invalid", &join(path, &format!("dhcp.{key}")), "DHCP endpoint must be a usable IPv4 host in the subscriber subnet.",
                "Choose a host address inside this network; exclude subnet and broadcast addresses.");
        }
    }
    if let (Some(start), Some(end)) = (start.map(u32::from), end.map(u32::from)) {
        if start > end {
            report.error(
                "dhcp.range.reversed",
                &join(path, "dhcp"),
                "DHCP range starts after it ends.",
                "Put range_start before or equal to range_end.",
            );
        } else {
            if gateway.is_some_and(|ip| (start..=end).contains(&u32::from(ip))) {
                report.error(
                    "dhcp.range.includes_gateway",
                    &join(path, "dhcp"),
                    "DHCP range includes the gateway address.",
                    "Exclude the gateway from dynamic leases.",
                );
            }
            if number(network, "session_limit") > u64::from(end) - u64::from(start) + 1 {
                report.error(
                    "capacity.leases.insufficient",
                    &join(path, "session_limit"),
                    "Session limit exceeds the dynamic lease count.",
                    "Reduce this profile's session limit or provide a larger valid DHCP range.",
                );
            }
        }
    }
    let servers = child(network, "dns")
        .get("upstream_servers")
        .expect("schema-checked")
        .get_ref()
        .as_array()
        .expect("schema-checked");
    if servers.is_empty() {
        report.error(
            "dns.servers.empty",
            &join(path, "dns.upstream_servers"),
            "At least one upstream DNS address is required.",
            "Configure a reachable IPv4 resolver; reachability is verified separately.",
        );
    }
    for (i, value) in servers.iter().enumerate() {
        let address = value.get_ref().as_str().expect("schema-checked");
        if !address
            .parse::<Ipv4Addr>()
            .is_ok_and(|ip| !ip.is_unspecified() && !ip.is_multicast() && !ip.is_broadcast())
        {
            report.error(
                "dns.address.invalid",
                &join(path, &format!("dns.upstream_servers[{i}]")),
                "Expected a unicast IPv4 DNS address.",
                "Use an IP address rather than a hostname, wildcard, or multicast destination.",
            );
        } else if address.starts_with("192.0.2.") {
            report.warning(
                "config.example.address",
                &join(path, &format!("dns.upstream_servers[{i}]")),
                "Documentation-only DNS address is configured.",
                "Replace the example resolver before deployment.",
            );
        }
    }
    gateway
}

fn host(net: Ipv4Net, ip: Ipv4Addr) -> bool {
    net.contains(&ip) && ip != net.network() && ip != net.broadcast()
}

fn accounting_config(config: &DeTable<'_>, path: &str, report: &mut Report) {
    match text(config, "sink") {
        "radius" => {
            if !optional_number(config, "interim_seconds").is_some_and(|n| n >= 60) {
                report.error("accounting.interval.invalid", &join(path, "interim_seconds"), "RADIUS interim interval must be at least 60 seconds.",
                    "Set interim_seconds to 60 or more; later runtime output must show any server override.");
            }
            for key in ["journal_path", "journal_max_bytes", "on_journal_full"] {
                unused(config, key, path, report);
            }
        }
        "local" => {
            file_path(
                report,
                &join(path, "journal_path"),
                optional_text(config, "journal_path"),
            );
            if optional_number(config, "journal_max_bytes").is_none_or(|n| n == 0) {
                report.error(
                    "accounting.journal.capacity",
                    &join(path, "journal_max_bytes"),
                    "A nonzero local journal size limit is required.",
                    "Specify the bounded disk budget in bytes.",
                );
            }
            required_choice(
                config,
                "on_journal_full",
                path,
                &["deny_new_sessions"],
                report,
            );
            unused(config, "interim_seconds", path, report);
        }
        _ => {}
    }
}

fn radius_pool(pool: &DeTable<'_>, path: &str, report: &mut Report) {
    let timeout = number(pool, "timeout_seconds");
    positive(report, &join(path, "timeout_seconds"), timeout);
    if timeout * (number(pool, "retries") + 1) > 60 {
        report.warning(
            "radius.retry_budget.long",
            path,
            "Configured timeout and attempts allow over 60 seconds before exhaustion.",
            "Check the acceptable login delay and backend retry load before deployment.",
        );
    }
    let servers = tables(pool, "servers");
    if servers.is_empty() {
        report.error(
            "radius.servers.empty",
            &join(path, "servers"),
            "RADIUS pool must contain at least one server.",
            "Add a named server endpoint to this pool.",
        );
    }
    let mut names = BTreeSet::new();
    let mut endpoints = BTreeSet::new();
    for (i, server) in servers.iter().enumerate() {
        let path = format!("{path}.servers[{i}]");
        let server_name = text(server, "name");
        name(report, &join(&path, "name"), server_name);
        if !names.insert(server_name) {
            report.error(
                "radius.server.duplicate_name",
                &join(&path, "name"),
                "Server name is duplicated within this pool.",
                "Give each endpoint a unique name.",
            );
        }
        let address = text(server, "address").parse::<IpAddr>().ok();
        if !address.is_some_and(|ip| {
            !ip.is_unspecified() && !ip.is_multicast() && ip != IpAddr::V4(Ipv4Addr::BROADCAST)
        }) {
            report.error("radius.address.invalid", &join(&path, "address"), "Expected a unicast RADIUS server IP address.",
                "Use an IPv4 or IPv6 address; hostname resolution is outside this version 1 configuration profile.");
        } else if text(server, "address").starts_with("192.0.2.") {
            report.warning(
                "config.example.address",
                &join(&path, "address"),
                "Documentation-only RADIUS address is configured.",
                "Replace the example endpoint before deployment.",
            );
        }
        choice(
            report,
            &join(&path, "transport"),
            text(server, "transport"),
            &["udp"],
        );
        for key in ["auth_port", "accounting_port", "priority"] {
            positive(report, &join(&path, key), number(server, key));
        }
        file_path(
            report,
            &join(&path, "secret_file"),
            Some(text(server, "secret_file")),
        );
        choice(
            report,
            &join(&path, "status_probe"),
            text(server, "status_probe"),
            &["disabled", "enabled"],
        );
        if let Some(address) = address {
            let endpoint = (
                address,
                number(server, "auth_port"),
                number(server, "accounting_port"),
            );
            if !endpoints.insert(endpoint) {
                report.error(
                    "radius.server.duplicate_endpoint",
                    &path,
                    "Same RADIUS IP/ports are repeated within a pool.",
                    "Configure distinct endpoints; repeating one does not provide failover.",
                );
            }
        }
    }
}

fn capacities(
    config: &DeTable<'_>,
    count: usize,
    sessions: u64,
    buffer: u64,
    radius: bool,
    report: &mut Report,
) {
    for key in [
        "max_networks",
        "max_sessions",
        "packet_buffers",
        "packet_buffer_bytes",
    ] {
        positive(report, &format!("capacity.{key}"), number(config, key));
    }
    if count == 0 {
        report.error(
            "network.empty",
            "networks",
            "At least one subscriber profile is required.",
            "Add an explicit tagged or untagged network profile.",
        );
    }
    if count as u64 > number(config, "max_networks") {
        report.error(
            "capacity.networks.exceeded",
            "capacity.max_networks",
            "Configured profiles exceed the network capacity.",
            "Increase max_networks or remove profiles.",
        );
    }
    if sessions > number(config, "max_sessions") {
        report.error("capacity.sessions.exceeded", "capacity.max_sessions", "Sum of per-network session ceilings exceeds the global capacity.",
            "Increase max_sessions or reduce profile session limits; version 1 does not overcommit these ceilings.");
    }
    if number(config, "packet_buffer_bytes") < buffer {
        report.error("capacity.packet_buffer.too_small", "capacity.packet_buffer_bytes", "Packet buffer cannot hold the largest configured Ethernet frame.",
            "Allow at least MTU plus 14 bytes, or MTU plus 18 for a VLAN tag, excluding FCS and additional backend headroom.");
    }
    if number(config, "shaping_queue_packets") >= number(config, "packet_buffers") {
        report.error("capacity.shaping.exhausts_buffers", "capacity.shaping_queue_packets", "Shaping queue can consume every shared packet buffer.",
            "Keep shaping_queue_packets below packet_buffers to leave capacity outside shaping queues.");
    }
    if radius && number(config, "pending_radius_requests") == 0 {
        report.error(
            "capacity.radius.zero",
            "capacity.pending_radius_requests",
            "RADIUS-backed profiles have no pending request capacity.",
            "Reserve a nonzero bounded request pool.",
        );
    }
}

pub(crate) fn name(report: &mut Report, field: &str, value: &str) {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        report.error(
            "config.name.invalid",
            field,
            "Name must be 1–64 ASCII letters, digits, hyphens or underscores.",
            "Use a short stable identifier for configuration references and diagnostics.",
        );
    }
}

fn interface(report: &mut Report, field: &str, value: &str) {
    if value.is_empty()
        || value.len() > 15
        || matches!(value, "." | "..")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
    {
        report.error(
            "interface.name.invalid",
            field,
            "Expected a simple Linux interface name of 1–15 ASCII bytes.",
            "Use the interface name; existence is checked later by doctor, not by config check.",
        );
    }
}

pub(crate) fn file_path(report: &mut Report, field: &str, value: Option<&str>) {
    if !value.is_some_and(|s| {
        Path::new(s).is_absolute() && s.len() > 1 && !s.chars().any(char::is_control)
    }) {
        report.error("config.path.invalid", field, "An absolute file or directory path is required.",
            "Set an absolute reference path; this command does not read the referenced file or its contents.");
    }
}

pub(crate) fn choice(report: &mut Report, field: &str, value: &str, supported: &[&str]) {
    if !supported.contains(&value) {
        report.error(
            "config.value.unsupported",
            field,
            "Value is unsupported in schema version 1.",
            &format!("Choose one of: {}.", supported.join(", ")),
        );
    }
}

fn required_choice(
    config: &DeTable<'_>,
    key: &str,
    path: &str,
    supported: &[&str],
    report: &mut Report,
) {
    if let Some(value) = optional_text(config, key) {
        choice(report, &join(path, key), value, supported);
    } else {
        report.error(
            "config.value.required",
            &join(path, key),
            "Field is required for the selected provider or sink.",
            &format!("Set this field to one of: {}.", supported.join(", ")),
        );
    }
}

pub(crate) fn unused(config: &DeTable<'_>, key: &str, path: &str, report: &mut Report) {
    if config.contains_key(key) {
        report.error(
            "config.value.inapplicable",
            &join(path, key),
            "Field does not apply to the selected mode.",
            "Remove this setting rather than relying on it being ignored.",
        );
    }
}

fn positive(report: &mut Report, field: &str, value: u64) {
    if value == 0 {
        report.error(
            "config.value.zero",
            field,
            "Value must be greater than zero.",
            "Set a nonzero bounded value for this resource or duration.",
        );
    }
}
