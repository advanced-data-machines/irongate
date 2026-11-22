//! Hosting and tenant namespace are independent of provider behavior.

use crate::Report;
use crate::schema::{join, optional_text, text};
use crate::validate::{choice, name, unused};
use toml::de::DeTable;

pub(super) fn effective(mode: &str) -> &'static str {
    if mode == "builtin_consent" {
        "local"
    } else {
        "external"
    }
}

pub(super) fn check(config: &DeTable<'_>, path: &str, report: &mut Report) {
    let mode = text(config, "mode");
    let expected = effective(mode);
    match optional_text(config, "hosting") {
        Some(value) if matches!(value, "local" | "external") => {
            if matches!(mode, "builtin_consent" | "external_uam") && value != expected {
                report.error("portal.hosting.conflict", &join(path, "hosting"),
                    "Portal hosting conflicts with its selected protocol mode.",
                    "Use local with builtin_consent, or external with external_uam; provider behavior is selected separately.");
            }
        }
        Some(value) => choice(report, &join(path, "hosting"), value, &["local", "external"]),
        None if matches!(mode, "builtin_consent" | "external_uam") => report.warning(
            "portal.hosting.implicit", &join(path, "hosting"),
            "Legacy schema-1 profile infers portal hosting from its mode.",
            "Set hosting explicitly to local (on this device) or external (separate LAN/cloud service)."),
        None => {}
    }
    let keys = ["tenant_id", "site_id", "gateway_id"];
    if mode != "external_uam" {
        for key in keys {
            unused(config, key, path, report);
        }
        return;
    }
    if !keys.iter().any(|key| config.contains_key(*key)) {
        return;
    }
    for key in keys {
        match optional_text(config, key) {
            Some(value) => name(report, &join(path, key), value),
            None => report.error("portal.scope.incomplete", &join(path, key),
                "Tenant-scoped portals require tenant, site and gateway identifiers together.",
                "Set tenant_id, site_id and gateway_id, or omit all three for an unscoped external service."),
        }
    }
    if optional_text(config, "url")
        .and_then(|url| url.parse::<http::Uri>().ok())
        .and_then(|uri| uri.scheme_str().map(str::to_owned))
        .as_deref()
        != Some("https")
    {
        report.error("portal.scope.https_required", &join(path, "url"),
            "Tenant-scoped external portals require HTTPS.",
            "Use an HTTPS portal URL; these identifiers are scope metadata, not authentication credentials.");
    }
}
