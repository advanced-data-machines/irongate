//! Portal contract validation, separate from host and provider selection.

mod hosting;

use crate::Report;
use crate::schema::{join, optional_text, text};
use crate::validate::{choice, file_path, unused};
use std::net::{Ipv4Addr, SocketAddrV4};
use toml::de::DeTable;

/// Redacted configured portal placement; does not establish runtime availability.
#[derive(Debug)]
pub struct PortalProfile {
    /// Subscriber network name.
    pub network: String,
    /// `local` means on the gateway; `external` means a separate service.
    pub hosting: &'static str,
    /// Explicit hosting field versus legacy mode-based inference.
    pub hosting_explicit: bool,
    /// Portal protocol/behavior, separate from the authentication provider.
    pub mode: String,
    /// Whether tenant/site/gateway scope metadata is configured; values omitted.
    pub tenant_scoped: bool,
}

pub(crate) fn profiles(root: &DeTable<'_>) -> Vec<PortalProfile> {
    crate::schema::tables(root, "networks")
        .iter()
        .map(|network| {
            let config = crate::schema::child(network, "portal");
            PortalProfile {
                network: text(network, "name").to_owned(),
                hosting: hosting::effective(text(config, "mode")),
                hosting_explicit: config.contains_key("hosting"),
                mode: text(config, "mode").to_owned(),
                tenant_scoped: config.contains_key("tenant_id"),
            }
        })
        .collect()
}

pub(crate) fn check(
    config: &DeTable<'_>,
    path: &str,
    gateway: Option<Ipv4Addr>,
    report: &mut Report,
) {
    hosting::check(config, path, report);
    match text(config, "mode") {
        "external_uam" => {
            file_path(
                report,
                &join(path, "secret_file"),
                optional_text(config, "secret_file"),
            );
            let url = optional_text(config, "url");
            let uri = url.and_then(|value| value.parse::<http::Uri>().ok());
            let valid = uri.as_ref().is_some_and(|uri| {
                matches!(uri.scheme_str(), Some("http" | "https"))
                    && uri.authority().is_some_and(valid_authority)
            }) && url.is_some_and(|value| !value.contains('#'));
            if !valid {
                report.error("portal.url.invalid", &join(path, "url"), "Expected an absolute HTTP(S) portal URI without credentials or fragments.",
                    "Set a portal URL with a host and a valid optional port; keep secrets in secret_file.");
            } else if uri.as_ref().and_then(|uri| uri.scheme_str()) == Some("http") {
                report.warning(
                    "portal.url.unencrypted",
                    &join(path, "url"),
                    "External portal uses unencrypted HTTP.",
                    "Use HTTPS for a deployed authentication portal.",
                );
            }
            for key in ["listen", "terms_file"] {
                unused(config, key, path, report);
            }
        }
        "builtin_consent" => {
            let listen =
                optional_text(config, "listen").and_then(|s| s.parse::<SocketAddrV4>().ok());
            if !listen
                .is_some_and(|addr| addr.port() != 0 && gateway.is_some_and(|ip| *addr.ip() == ip))
            {
                report.error("portal.listen.invalid", &join(path, "listen"), "Built-in portal must bind the subscriber gateway IPv4 address and a nonzero port.",
                    "Use this network's gateway:port; wildcard and management bindings are not accepted.");
            }
            file_path(
                report,
                &join(path, "terms_file"),
                optional_text(config, "terms_file"),
            );
            for key in ["url", "secret_file"] {
                unused(config, key, path, report);
            }
        }
        mode => choice(
            report,
            &join(path, "mode"),
            mode,
            &["external_uam", "builtin_consent"],
        ),
    }
}

fn valid_authority(authority: &http::uri::Authority) -> bool {
    if authority.host().is_empty() || authority.as_str().contains('@') {
        return false;
    }
    match authority.port() {
        Some(port) => port.as_str().parse::<u16>().is_ok_and(|n| n > 0),
        // http::Authority permits ports that cannot be represented as u16 and
        // returns None for them. Distinguish that from an absent port, including
        // the colons inside a bracketed IPv6 host.
        None if authority.as_str().starts_with('[') => authority.as_str().ends_with(']'),
        None => !authority.as_str().contains(':'),
    }
}
