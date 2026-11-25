//! One native UDP worker; retries are scheduled by the owner, never per-packet.
use super::{Dispatch, Receipt, config::Config, model::Outcome, wire};
use crate::lab::radius::client::Channel;
use std::{
    io,
    net::{SocketAddrV4, UdpSocket},
    os::unix::net::UnixDatagram,
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
pub(crate) fn start(
    config: Option<Arc<Config>>,
    requests: Receiver<Dispatch>,
    replies: SyncSender<Receipt>,
    wake: &UnixDatagram,
) -> io::Result<()> {
    if let Some(config) = config {
        let wake = wake.try_clone()?;
        std::thread::spawn(move || run(config, requests, replies, wake));
    }
    Ok(())
}
fn run(
    config: Arc<Config>,
    requests: Receiver<Dispatch>,
    replies: SyncSender<Receipt>,
    wake: UnixDatagram,
) {
    for dispatch in requests {
        let receipt = deliver(&config, dispatch);
        if replies.send(receipt).is_err() {
            return;
        }
        let _ = wake.send(&[1]);
    }
}
fn deliver(config: &Config, dispatch: Dispatch) -> Receipt {
    let result = || -> io::Result<Receipt> {
        let mut packet = zeroize::Zeroizing::new([0; 512]);
        let len = wire::encode(config, dispatch, dispatch.token.sequence as u8, &mut packet);
        let socket = UdpSocket::bind(SocketAddrV4::new(crate::lab::profile::WAN_IP.into(), 0))?;
        socket.connect(config.endpoint)?;
        socket.set_write_timeout(Some(Duration::from_millis(200)))?;
        Ok(exchange(
            &socket,
            &packet[..len],
            &config.radius.secret,
            dispatch,
            Instant::now,
        ))
    };
    result().unwrap_or(Receipt {
        token: dispatch.token,
        outcome: Outcome::Unavailable,
        invalid: 0,
    })
}
pub(super) fn exchange(
    socket: &impl Channel,
    request: &[u8],
    secret: &[u8],
    dispatch: Dispatch,
    now: impl Fn() -> Instant,
) -> Receipt {
    let mut receipt = Receipt {
        token: dispatch.token,
        outcome: Outcome::Timeout,
        invalid: 0,
    };
    if socket.send(request).ok() != Some(request.len()) {
        receipt.outcome = Outcome::Unavailable;
        return receipt;
    }
    let deadline = now() + Duration::from_secs(1);
    let mut input = [0; 4097];
    // A malicious reply flood has a fixed receive-work limit as well as a deadline.
    for _ in 0..64 {
        let wait = deadline
            .saturating_duration_since(now())
            .min(Duration::from_millis(150));
        if wait.is_zero() {
            break;
        }
        match socket.receive(&mut input, wait) {
            Ok(len) if wire::acknowledged(&input[..len], request, secret) => {
                receipt.outcome = Outcome::Ack;
                return receipt;
            }
            Ok(_) => receipt.invalid = receipt.invalid.saturating_add(1),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => {
                receipt.outcome = Outcome::Unavailable;
                return receipt;
            }
        }
    }
    if receipt.invalid > 0 {
        receipt.outcome = Outcome::Invalid;
    }
    receipt
}
