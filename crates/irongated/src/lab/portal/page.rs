//! Self-contained local page; no script/CDN assets and no identifiers in URLs.

use super::{
    bridge::{Reply, Snapshot},
    protocol::encode,
    tokens::Action,
};
use serde_json::{Value, json};

pub(super) fn status(view: Snapshot) -> Value {
    json!({"scope":"local_lab_consent","hosting":"local","provider":"consent","network":1,
        "transport":view.transport.map(|h|json!({"scheme":if h.secure{"https"}else{"http"},"tls_handshakes":h.completed,"tls_failures":h.failed,"tls_timeouts":h.timed_out,"accepted":h.accepted,"queue_rejections":h.busy})),
        "phase":view.phase,"now":view.now,"allowed":view.allowed,"generation":view.generation,"until":view.until,
        "forwarded_ip_bytes":view.ip_bytes,"admission_available":view.admission_available,
        "records":{"retained":view.records,"durable":view.durable,"pending":view.records-view.durable,
            "write_failures":view.record_failures,"capacity":view.record_capacity,"stalled":view.record_stalled,
            "oldest_pending_seconds":view.pending_age,"crash_guarantee":"acknowledged_prefix_only"},
        "action":view.action.map(|(kind,token,until)|json!({"kind":if kind==Action::Consent{"consent"}else{"logout"},"token":encode(token),"expires_at":until}))})
}

pub(super) fn response(reply: Reply, html: bool) -> Vec<u8> {
    let (code, kind, body, location) = match reply {
        Reply::View(view) if html => (200, "text/html; charset=utf-8", render(view), false),
        Reply::View(view) => (200, "application/json", status(view).to_string(), false),
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
    let title = if view.allowed {
        "You’re connected"
    } else {
        "Welcome to the network"
    };
    let explanation = if view.allowed {
        "Your temporary network access is active. You can end it at any time."
    } else {
        "Accept the test network terms to connect. No personal login is required."
    };
    let form = match view.action {
        Some((kind, token, _)) => {
            let (path, label) = if kind == Action::Consent {
                ("consent", "Accept and connect")
            } else {
                ("logout", "Disconnect")
            };
            let disabled = if kind == Action::Consent && !view.admission_available {
                " disabled"
            } else {
                ""
            };
            format!(
                "<form method=post action=/{path}><input type=hidden name=token value=\"{}\"><button{disabled}>{label}</button></form>",
                encode(token)
            )
        }
        None => String::new(),
    };
    let warning = if !view.allowed && !view.admission_available {
        "<p class=warning>The local activity log is full or unavailable. New connections are paused.</p>"
    } else {
        ""
    };
    format!(
        "<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content=\"width=device-width, initial-scale=1\"><title>IronGate — Network access</title><script src=/portal.js defer></script><style>{}</style><body data-allowed={}><main><header>IronGate <span>LOCAL NETWORK</span></header><div class=badge>{}</div><h1>{title}</h1><p>{explanation}</p><section><h2>Test network terms</h2><p>Please use this test network responsibly. Access is temporary; accepting these terms does not verify a person’s identity.</p></section>{warning}{form}<footer>Session {} · {}<br><span id=records>Local records: {} retained, {} saved.</span> Unacknowledged records can be lost on a crash.<br>Refresh this page if an action expires.</footer></main></html>",
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
