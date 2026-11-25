//! Shared native transport dispatch, exercised with anonymous sockets in tests.
use super::{
    bridge::Envelope,
    config::Settings,
    metrics::Metrics,
    server,
    transport::{Secure, Socket, TimedIo},
};
use std::{
    io,
    net::Ipv4Addr,
    sync::{Arc, atomic::Ordering::Relaxed, mpsc::SyncSender},
};
pub(super) fn handle<T: TimedIo>(
    socket: Socket<T>,
    peer: Ipv4Addr,
    settings: &Settings,
    sender: &SyncSender<Envelope>,
    notify: impl Fn(),
    metrics: &Metrics,
) -> io::Result<()> {
    if let Some(config) = &settings.tls {
        let conn = rustls::ServerConnection::new(Arc::clone(config)).map_err(io::Error::other)?;
        let mut stream = Secure { conn, socket };
        if let Err(error) = stream.handshake() {
            if error.kind() == io::ErrorKind::TimedOut {
                metrics.timed_out.fetch_add(1, Relaxed);
            } else {
                metrics.failed.fetch_add(1, Relaxed);
            }
            return Err(error);
        }
        metrics.completed.fetch_add(1, Relaxed);
        server::handle(&mut stream, peer, sender, notify, settings, metrics)?;
        stream.conn.send_close_notify();
        std::io::Write::flush(&mut stream)
    } else {
        let mut stream = socket;
        server::handle(&mut stream, peer, sender, notify, settings, metrics)
    }
}
