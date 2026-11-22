//! Schema checks over the parser's borrowed, source-spanned representation.

use crate::{Report, locate_last};
use toml::de::{DeTable, DeValue};

enum Kind {
    Text,
    Unsigned(u64),
    Table(&'static [Field]),
    Tables(&'static [Field]),
    Texts,
}

struct Field {
    name: &'static str,
    kind: Kind,
    required: bool,
}

macro_rules! field {
    ($name:literal, $kind:expr) => {
        Field {
            name: $name,
            kind: $kind,
            required: true,
        }
    };
    ($name:literal, $kind:expr, optional) => {
        Field {
            name: $name,
            kind: $kind,
            required: false,
        }
    };
}

use Kind::{Table, Tables, Text, Texts, Unsigned};
const COUNT: u64 = u32::MAX as u64;
const GATEWAY: &[Field] = &[
    field!("name", Text),
    field!("platform_mode", Text),
    field!("forwarding", Text),
    field!("unmatched_subscriber_traffic", Text),
];
const INTERFACES: &[Field] = &[field!("upstream", Text)];
const IPV4: &[Field] = &[field!("subnet", Text), field!("gateway", Text)];
const DHCP: &[Field] = &[
    field!("mode", Text),
    field!("range_start", Text),
    field!("range_end", Text),
    field!("lease_seconds", Unsigned(COUNT)),
];
const DNS: &[Field] = &[field!("upstream_servers", Texts)];
const PORTAL: &[Field] = &[
    field!("hosting", Text, optional),
    field!("mode", Text),
    field!("tenant_id", Text, optional),
    field!("site_id", Text, optional),
    field!("gateway_id", Text, optional),
    field!("url", Text, optional),
    field!("secret_file", Text, optional),
    field!("listen", Text, optional),
    field!("terms_file", Text, optional),
];
const AUTH: &[Field] = &[
    field!("provider", Text),
    field!("radius_pool", Text, optional),
];
const ACCOUNTING: &[Field] = &[
    field!("sink", Text),
    field!("radius_pool", Text, optional),
    field!("interim_seconds", Unsigned(COUNT), optional),
    field!("journal_path", Text, optional),
    field!("journal_max_bytes", Unsigned(u64::MAX), optional),
    field!("on_journal_full", Text, optional),
];
const POLICY: &[Field] = &[
    field!("radius_unavailable", Text, optional),
    field!("existing_sessions_on_radius_outage", Text, optional),
    field!("session_timeout_seconds", Unsigned(COUNT)),
    field!("idle_timeout_seconds", Unsigned(COUNT)),
];
const NETWORK: &[Field] = &[
    field!("name", Text),
    field!("interface", Text),
    field!("tagging", Text),
    field!("vlan_id", Unsigned(u16::MAX as u64), optional),
    field!("session_limit", Unsigned(COUNT)),
    field!("mtu", Unsigned(COUNT)),
    field!("ipv6", Text),
    field!("ipv4", Table(IPV4)),
    field!("dhcp", Table(DHCP)),
    field!("dns", Table(DNS)),
    field!("portal", Table(PORTAL)),
    field!("authentication", Table(AUTH)),
    field!("accounting", Table(ACCOUNTING)),
    field!("policy", Table(POLICY)),
];
const SERVER: &[Field] = &[
    field!("name", Text),
    field!("address", Text),
    field!("transport", Text),
    field!("auth_port", Unsigned(u16::MAX as u64)),
    field!("accounting_port", Unsigned(u16::MAX as u64)),
    field!("secret_file", Text),
    field!("priority", Unsigned(u16::MAX as u64)),
    field!("status_probe", Text),
];
const POOL: &[Field] = &[
    field!("name", Text),
    field!("timeout_seconds", Unsigned(COUNT)),
    field!("retries", Unsigned(u8::MAX as u64)),
    field!("servers", Tables(SERVER)),
];
const RADIUS: &[Field] = &[field!("pools", Tables(POOL))];
const CAPACITY: &[Field] = &[
    field!("max_networks", Unsigned(COUNT)),
    field!("max_sessions", Unsigned(COUNT)),
    field!("packet_buffers", Unsigned(COUNT)),
    field!("packet_buffer_bytes", Unsigned(COUNT)),
    field!("pending_radius_requests", Unsigned(COUNT)),
    field!("shaping_queue_packets", Unsigned(COUNT)),
];
const ROOT: &[Field] = &[
    field!("schema_version", Unsigned(COUNT)),
    field!("gateway", Table(GATEWAY)),
    field!("interfaces", Table(INTERFACES)),
    field!("networks", Tables(NETWORK)),
    field!("radius", Table(RADIUS), optional),
    field!("capacity", Table(CAPACITY)),
];

pub(crate) fn check(table: &DeTable<'_>, input: &str, report: &mut Report) {
    check_table(table, ROOT, "", input, report);
}

fn check_table(
    table: &DeTable<'_>,
    fields: &[Field],
    parent: &str,
    input: &str,
    report: &mut Report,
) {
    for (key, value) in table.iter() {
        let name = key.get_ref().as_ref();
        let path = join(parent, name);
        let Some(field) = fields.iter().find(|field| field.name == name) else {
            report.error(
                "config.schema.unknown_field",
                &path,
                "Unknown configuration field.",
                "Remove or correct this key; use the schema version 1 examples as a reference.",
            );
            locate_last(report, input, Some(key.span()));
            continue;
        };
        let valid_type = match (&field.kind, value.get_ref()) {
            (Text, DeValue::String(_)) => true,
            (Unsigned(max), value @ DeValue::Integer(_)) => {
                if unsigned(value).is_some_and(|n| n <= *max) {
                    true
                } else {
                    report.error(
                        "config.schema.integer_range",
                        &path,
                        "Integer is outside the field's unsigned range.",
                        &format!("Use an integer between 0 and {max}."),
                    );
                    locate_last(report, input, Some(value_span(table, name)));
                    continue;
                }
            }
            (Table(children), DeValue::Table(child)) => {
                check_table(child, children, &path, input, report);
                true
            }
            (Tables(children), DeValue::Array(array)) => {
                for (index, entry) in array.iter().enumerate() {
                    let item_path = format!("{path}[{index}]");
                    if let DeValue::Table(child) = entry.get_ref() {
                        check_table(child, children, &item_path, input, report);
                    } else {
                        report.error(
                            "config.schema.invalid_type",
                            &item_path,
                            "Expected a table in this array.",
                            "Use an array of TOML tables, such as [[networks]].",
                        );
                        locate_last(report, input, Some(entry.span()));
                    }
                }
                true
            }
            (Texts, DeValue::Array(array)) => {
                for (index, entry) in array.iter().enumerate() {
                    if !matches!(entry.get_ref(), DeValue::String(_)) {
                        report.error(
                            "config.schema.invalid_type",
                            &format!("{path}[{index}]"),
                            "Expected a quoted string in this array.",
                            "Use a quoted IP address string.",
                        );
                        locate_last(report, input, Some(entry.span()));
                    }
                }
                true
            }
            _ => false,
        };
        if !valid_type {
            report.error("config.schema.invalid_type", &path, "Invalid field type or structure.",
                "Check the example: quote text, use integer counts, and preserve table/array structure.");
            locate_last(report, input, Some(value.span()));
        }
    }
    for field in fields {
        if field.required && !table.contains_key(field.name) {
            report.error(
                "config.schema.missing_field",
                &join(parent, field.name),
                "Required configuration field is missing.",
                "Add this field to the indicated table.",
            );
        }
    }
}

fn value_span(table: &DeTable<'_>, key: &str) -> std::ops::Range<usize> {
    table.get(key).expect("key being validated exists").span()
}

pub(crate) fn join(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

pub(crate) fn unsigned(value: &DeValue<'_>) -> Option<u64> {
    let number = value.as_integer()?;
    i128::from_str_radix(number.as_str(), number.radix())
        .ok()?
        .try_into()
        .ok()
}

// These accessors are used only after the complete schema has been checked.
pub(crate) fn text<'a>(table: &'a DeTable<'_>, key: &str) -> &'a str {
    optional_text(table, key).expect("required string was schema-checked")
}

pub(crate) fn optional_text<'a>(table: &'a DeTable<'_>, key: &str) -> Option<&'a str> {
    table.get(key)?.get_ref().as_str()
}

pub(crate) fn number(table: &DeTable<'_>, key: &str) -> u64 {
    optional_number(table, key).expect("required unsigned integer was schema-checked")
}

pub(crate) fn optional_number(table: &DeTable<'_>, key: &str) -> Option<u64> {
    unsigned(table.get(key)?.get_ref())
}

pub(crate) fn child<'a, 'i>(table: &'a DeTable<'i>, key: &str) -> &'a DeTable<'i> {
    table
        .get(key)
        .and_then(|value| value.get_ref().as_table())
        .expect("required table was schema-checked")
}

pub(crate) fn tables<'a, 'i>(table: &'a DeTable<'i>, key: &str) -> Vec<&'a DeTable<'i>> {
    table
        .get(key)
        .and_then(|value| value.get_ref().as_array())
        .expect("required array was schema-checked")
        .iter()
        .map(|value| {
            value
                .get_ref()
                .as_table()
                .expect("table array was schema-checked")
        })
        .collect()
}
