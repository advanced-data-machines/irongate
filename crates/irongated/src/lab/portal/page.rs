//! Self-contained local page; no script/CDN assets and no identifiers in URLs.

use super::{
    bridge::{Reply, Snapshot},
    protocol::encode,
    tokens::Action,
};
use serde_json::{Value, json};

pub(super) fn status(view: Snapshot) -> Value {
    let mut authentication = view.authentication.status(
        view.provider == irongate_core::session::Provider::Radius,
        view.now,
    );
    if view.accounting.enabled {
        authentication["accounting"] = "reported_separately".into();
    }
    json!({"scope":"local_lab_consent","hosting":"local","provider":if view.provider==irongate_core::session::Provider::Radius{"radius"}else{"consent"},
        "policy_source":view.policy_source.map(|p|match p{irongate_core::session::Provider::Consent=>"consent",irongate_core::session::Provider::Radius=>"radius"}),
        "authentication":authentication,"network":1,
        "accounting":view.accounting.json(view.now),
        "transport":view.transport.map(|h|json!({"scheme":if h.secure{"https"}else{"http"},"tls_handshakes":h.completed,"tls_failures":h.failed,"tls_timeouts":h.timed_out,"accepted":h.accepted,"queue_rejections":h.busy})),
        "phase":view.phase,"now":view.now,"allowed":view.allowed,"generation":view.generation,"until":view.until,
        "forwarded_ip_bytes":view.ip_bytes,"admission_available":view.admission_available,
        "records":{"retained":view.records,"durable":view.durable,"pending":view.records-view.durable,
            "write_failures":view.record_failures,"capacity":view.record_capacity,"stalled":view.record_stalled,
            "oldest_pending_seconds":view.pending_age,"crash_guarantee":"acknowledged_prefix_only"},
        "action":view.action.map(|(kind,token,until)|json!({"kind":match kind{Action::Consent=>"consent",Action::Login=>"login",Action::Logout=>"logout"},"token":encode(token),"expires_at":until}))})
}

pub(super) fn response(reply: Reply, html: bool) -> Vec<u8> {
    let (code, kind, body, location) = match reply {
        Reply::View(view) if html => (200, "text/html; charset=utf-8", render(view), false),
        Reply::View(view) => (200, "application/json", status(view).to_string(), false),
        Reply::Authenticate { .. } => (
            500,
            "application/json",
            r#"{"error":"Internal authentication boundary failure"}"#.to_owned(),
            false,
        ),
        Reply::Script => (
            200,
            "text/javascript; charset=utf-8",
            include_str!("portal.js").to_owned(),
            false,
        ),
        Reply::Error(code, message) if html => (
            code,
            "text/html; charset=utf-8",
            format!(
                "<!doctype html><html lang=en><meta charset=utf-8><title>IronGate</title><style>{}</style><main><header>IronGate</header><h1>Please try again</h1><p>{message}</p><a href=/ >Return to the portal</a></main></html>",
                include_str!("portal.css")
            ),
            false,
        ),
        Reply::Redirect => (
            303,
            "text/plain; charset=utf-8",
            "Continue at the portal.".to_owned(),
            true,
        ),
        Reply::Error(code, message) => (
            code,
            "application/json",
            json!({"error":message}).to_string(),
            false,
        ),
    };
    let redirect = if location { "Location: /\r\n" } else { "" };
    format!("HTTP/1.1 {code} Result\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nReferrer-Policy: same-origin\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; connect-src 'self'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'\r\n{redirect}\r\n{body}",body.len()).into_bytes()
}

fn render(view: Snapshot) -> String {
    let radius = view.provider == irongate_core::session::Provider::Radius;
    let title = if view.allowed {
        "You’re connected"
    } else if radius {
        "Sign in to the network"
    } else {
        "Welcome to the network"
    };
    let explanation = if view.allowed {
        "Your temporary network access is active. You can end it at any time."
    } else if radius {
        "Use your network account to connect. Your sign-in is protected by HTTPS."
    } else {
        "Accept the test network terms to connect. No personal login is required."
    };
    let form = match view.action {
        Some((kind, token, _)) => {
            let (path, label) = if kind == Action::Consent {
                ("consent", "Accept and connect")
            } else if kind == Action::Login {
                ("login", "Sign in")
            } else {
                (
                    "logout",
                    if view.phase == "AwaitingAuthentication" {
                        "Cancel sign-in"
                    } else {
                        "Disconnect"
                    },
                )
            };
            let disabled = if kind != Action::Logout && !view.admission_available {
                " disabled"
            } else {
                ""
            };
            let fields = if kind == Action::Login {
                r#"<label>Username<input name=username autocomplete=username maxlength=64 required></label><label>Password<input type=password name=password autocomplete=current-password maxlength=128 required></label>"#
            } else {
                ""
            };
            format!(
                "<form method=post action=/{path}><input type=hidden name=token value=\"{}\">{fields}<button{disabled}>{label}</button></form>",
                encode(token)
            )
        }
        None => String::new(),
    };
    let warning = if !view.allowed && !view.admission_available {
        "<p class=warning>Local records or accounting delivery are unavailable. New connections are paused.</p>"
    } else {
        ""
    };
    let section_title = if radius {
        "Network access policy"
    } else {
        "Test network terms"
    };
    let terms = if radius {
        "Access is temporary and subject to your network account policy."
    } else {
        "Please use this test network responsibly. Access is temporary; accepting these terms does not verify a person’s identity."
    };
    format!(
        "<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\"><title>IronGate — Network access</title><script src=/portal.js defer></script><style>{}</style><body data-allowed={}><main><header>IronGate <span>LOCAL NETWORK</span></header><div class=badge>{}</div><h1>{title}</h1><p>{explanation}</p><section><h2>{section_title}</h2><p>{terms}</p></section>{warning}{form}<footer>Session {} · {}<br><span id=records>Local records: {} retained, {} saved.</span> Unacknowledged records can be lost on a crash.<br>Refresh this page if an action expires.</footer></main></html>",
        include_str!("portal.css"),
        view.allowed,
        if view.allowed {
            "Connected"
        } else {
            "Access paused"
        },
        view.generation,
        view.phase,
        view.records,
        view.durable
    )
}
