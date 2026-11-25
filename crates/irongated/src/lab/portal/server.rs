//! Fixed HTTP workers/queue with bounded request sizes and total deadlines.
use super::{
    bridge::{Envelope, Reply, WorkerBridge},
    page,
    protocol::{self, Parsed},
};
use std::{
    io::{self, Read},
    net::Ipv4Addr,
    sync::mpsc::SyncSender,
    time::{Duration, Instant},
};

pub(crate) fn start(
    bridge: WorkerBridge,
    run_id: &str,
) -> io::Result<irongate_core::session::Provider> {
    let settings = super::config::Settings::load(std::path::Path::new(
        "/run/irongate-packet-lab/portal.toml",
    ))?;
    let provider = if settings.radius.is_some() {
        irongate_core::session::Provider::Radius
    } else {
        irongate_core::session::Provider::Consent
    };
    super::workers::start(bridge, run_id, settings)?;
    Ok(provider)
}

pub(super) fn handle(
    stream: &mut impl super::transport::RequestStream,
    peer: Ipv4Addr,
    sender: &SyncSender<Envelope>,
    notify: impl Fn(),
    settings: &super::config::Settings,
    metrics: &super::metrics::Metrics,
) -> io::Result<()> {
    let result =
        read_request(stream, &settings.endpoint).and_then(|(request, html, credentials)| {
            let reply = super::login::ask(sender, peer, request, &notify)?;
            super::login::complete(reply, credentials, settings, peer, sender, notify)
                .map(|reply| (reply, html))
        });
    let (mut reply, html) = result.unwrap_or_else(|code| {
        (
            Reply::Error(code, "Request could not be completed. Reload the portal."),
            false,
        )
    });
    if let Reply::View(view) = &mut reply {
        view.transport = Some(metrics.snapshot(settings.endpoint.secure));
    }
    stream.begin_response();
    stream.write_all(&page::response(reply, html))?;
    stream.flush()
}
fn read_request(
    stream: &mut impl Read,
    endpoint: &super::config::Endpoint,
) -> Result<
    (
        super::bridge::Request,
        bool,
        Option<crate::lab::radius::credentials::Credentials>,
    ),
    u16,
> {
    let entropy = crate::lab::entropy::bytes().map_err(|_| 503u16)?;
    let start = Instant::now();
    let mut input = zeroize::Zeroizing::new([0; protocol::MAX_REQUEST]);
    let mut used = 0;
    loop {
        match protocol::parse(&input[..used], entropy, endpoint)? {
            Parsed::Ready {
                request,
                html,
                credentials,
            } => return Ok((request, html, credentials)),
            Parsed::Incomplete => (),
        }
        if used == input.len() {
            return Err(413);
        }
        if start.elapsed() >= Duration::from_secs(2) {
            return Err(408);
        }
        match stream.read(&mut input[used..]) {
            Ok(0) => return Err(400),
            Ok(length) => used += length,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(_) => return Err(400),
        }
    }
}
