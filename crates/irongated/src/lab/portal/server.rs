//! Fixed HTTP workers/queue with bounded request sizes and total deadlines.
use super::{
    bridge::{Envelope, Reply, WorkerBridge},
    page,
    protocol::{self, Parsed},
};
use rustix::rand::{GetRandomFlags, getrandom};
use std::{
    io::{self, Read},
    net::Ipv4Addr,
    sync::mpsc::{self, SyncSender},
    time::{Duration, Instant},
};

pub(crate) fn start(bridge: WorkerBridge, run_id: &str) -> io::Result<()> {
    let settings = super::config::Settings::load(std::path::Path::new(
        "/run/irongate-packet-lab/portal.toml",
    ))?;
    super::workers::start(bridge, run_id, settings)
}

pub(super) fn handle(
    stream: &mut impl super::transport::RequestStream,
    peer: Ipv4Addr,
    sender: &SyncSender<Envelope>,
    notify: impl FnOnce(),
    endpoint: &super::config::Endpoint,
    metrics: &super::metrics::Metrics,
) -> io::Result<()> {
    let result = read_request(stream, endpoint).and_then(|(request, html)| {
        let (reply, receiver) = mpsc::sync_channel(1);
        sender
            .try_send(Envelope {
                peer,
                request,
                reply,
            })
            .map_err(|_| 503u16)?;
        notify();
        receiver
            .recv_timeout(Duration::from_secs(2))
            .map(|reply| (reply, html))
            .map_err(|_| 503u16)
    });
    let (mut reply, html) = result.unwrap_or_else(|code| {
        (
            Reply::Error(code, "Request could not be completed. Reload the portal."),
            false,
        )
    });
    if let Reply::View(view) = &mut reply {
        view.transport = Some(metrics.snapshot(endpoint.secure));
    }
    stream.begin_response();
    stream.write_all(&page::response(reply, html))?;
    stream.flush()
}
fn read_request(
    stream: &mut impl Read,
    endpoint: &super::config::Endpoint,
) -> Result<(super::bridge::Request, bool), u16> {
    let entropy = entropy().map_err(|_| 503u16)?;
    let start = Instant::now();
    let mut input = [0; protocol::MAX_REQUEST];
    let mut used = 0;
    loop {
        match protocol::parse(&input[..used], entropy, endpoint)? {
            Parsed::Ready { request, html } => return Ok((request, html)),
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
fn entropy() -> io::Result<[u8; 32]> {
    let mut value = [0; 32];
    let mut used = 0;
    for _ in 0..8 {
        match getrandom(&mut value[used..], GetRandomFlags::NONBLOCK) {
            Ok(0) => break,
            Ok(length) => {
                used += length;
                if used == value.len() {
                    return Ok(value);
                }
            }
            Err(rustix::io::Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::other("OS entropy unavailable"))
}
