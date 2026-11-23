//! Fixed HTTP workers/queue with bounded request sizes and total deadlines.
use super::{
    bridge::{Envelope, Reply, WorkerBridge},
    page,
    protocol::{self, Parsed},
    writer,
};
use rustix::rand::{GetRandomFlags, getrandom};
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender},
    },
    thread,
    time::{Duration, Instant},
};

pub(crate) fn start(bridge: WorkerBridge, run_id: &str) -> io::Result<()> {
    let file = writer::open()?;
    let listener = TcpListener::bind("10.50.10.1:8080")?;
    let (connections, receiver) = mpsc::sync_channel::<TcpStream>(4);
    let receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..2 {
        let receiver = Arc::clone(&receiver);
        let sender = bridge.sender.clone();
        let notify = bridge.wake.try_clone()?;
        thread::spawn(move || {
            loop {
                let connection = {
                    let Ok(guard) = receiver.lock() else {
                        return;
                    };
                    guard.recv()
                };
                let Ok(mut stream) = connection else {
                    return;
                };
                let peer = match stream.peer_addr() {
                    Ok(std::net::SocketAddr::V4(peer)) => *peer.ip(),
                    _ => continue,
                };
                if stream
                    .set_read_timeout(Some(Duration::from_millis(200)))
                    .is_err()
                    || stream
                        .set_write_timeout(Some(Duration::from_millis(200)))
                        .is_err()
                {
                    continue;
                }
                let _ = handle(&mut stream, peer, &sender, || {
                    let _ = notify.send(&[1]);
                });
            }
        });
    }
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                break;
            };
            if let Err(mpsc::TrySendError::Full(returned)) = connections.try_send(stream) {
                stream = returned;
                let _ = stream.set_write_timeout(Some(Duration::from_millis(50)));
                let _ = stream.write_all(&page::response(
                    Reply::Error(503, "Portal is busy. Please retry."),
                    false,
                ));
            }
        }
    });
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

pub(super) fn handle(
    stream: &mut (impl Read + Write),
    peer: Ipv4Addr,
    sender: &SyncSender<Envelope>,
    notify: impl FnOnce(),
) -> io::Result<()> {
    let result = read_request(stream).and_then(|(request, html)| {
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
    let (reply, html) = result.unwrap_or_else(|code| {
        (
            Reply::Error(code, "Request could not be completed. Reload the portal."),
            false,
        )
    });
    stream.write_all(&page::response(reply, html))
}
fn read_request(stream: &mut impl Read) -> Result<(super::bridge::Request, bool), u16> {
    let entropy = entropy().map_err(|_| 503u16)?;
    let start = Instant::now();
    let mut input = [0; protocol::MAX_REQUEST];
    let mut used = 0;
    loop {
        match protocol::parse(&input[..used], entropy)? {
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
