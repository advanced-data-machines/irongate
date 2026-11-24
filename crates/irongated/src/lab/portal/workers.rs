//! Fixed service workers; the acceptor never performs a TLS handshake.
use super::{
    bridge::{Envelope, WorkerBridge},
    config::Settings,
    metrics::Metrics,
    transport::Socket,
    writer,
};
use std::sync::atomic::Ordering::Relaxed;
use std::{
    io,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

type Accepted = (TcpStream, Instant);
struct Worker {
    settings: Settings,
    metrics: Arc<Metrics>,
    sender: mpsc::SyncSender<Envelope>,
    wake: std::os::unix::net::UnixDatagram,
}
pub(super) fn start(bridge: WorkerBridge, run_id: &str, settings: Settings) -> io::Result<()> {
    let file = writer::open()?;
    let listener = TcpListener::bind(settings.endpoint.address)?;
    let (connections, receiver) = mpsc::sync_channel::<Accepted>(4);
    let receiver = Arc::new(Mutex::new(receiver));
    let metrics = Arc::new(Metrics::default());
    let worker = Arc::new(Worker {
        settings,
        metrics: Arc::clone(&metrics),
        sender: bridge.sender,
        wake: bridge.wake.try_clone()?,
    });
    for _ in 0..2 {
        let worker = Arc::clone(&worker);
        let receiver = Arc::clone(&receiver);
        thread::spawn(move || serve_queue(receiver, &worker));
    }
    thread::spawn(move || accept(listener, connections, &metrics));
    let run_id = run_id.to_owned();
    thread::spawn(move || {
        writer::run(
            file,
            bridge.records,
            bridge.record_acks,
            bridge.wake,
            run_id,
        )
    });
    Ok(())
}
fn accept(listener: TcpListener, queue: mpsc::SyncSender<Accepted>, metrics: &Metrics) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { break };
        metrics.accepted.fetch_add(1, Relaxed);
        if queue.try_send((stream, Instant::now())).is_err() {
            // Drop busy connections. Sending cleartext on a TLS listener would
            // be a protocol violation; handshaking here would stall acceptance.
            metrics.busy.fetch_add(1, Relaxed);
        }
    }
}
fn serve_queue(receiver: Arc<Mutex<mpsc::Receiver<Accepted>>>, worker: &Worker) {
    loop {
        let connection = {
            let Ok(receiver) = receiver.lock() else {
                return;
            };
            receiver.recv()
        };
        let Ok((stream, accepted)) = connection else {
            return;
        };
        let _ = worker.serve(stream, accepted);
    }
}
impl Worker {
    fn serve(&self, stream: TcpStream, accepted: Instant) -> io::Result<()> {
        let peer = match stream.peer_addr()? {
            SocketAddr::V4(peer) => *peer.ip(),
            _ => return Err(io::Error::other("IPv4 lab only")),
        };
        let socket = Socket::new(stream, accepted + Duration::from_secs(2));
        super::service::handle(
            socket,
            peer,
            &self.settings,
            &self.sender,
            || {
                let _ = self.wake.send(&[1]);
            },
            &self.metrics,
        )
    }
}
