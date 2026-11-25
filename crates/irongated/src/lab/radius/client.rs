//! Fixed datagram buffers and one retransmission within a total two-second budget.
use super::{Decision, Outcome, config::Config, credentials::Credentials, request, response};
use std::{
    io,
    net::{SocketAddrV4, UdpSocket},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;
pub(crate) fn authenticate(config: &Config, credentials: &Credentials) -> Decision {
    let started = Instant::now();
    let operation = || -> io::Result<Decision> {
        let entropy = crate::lab::entropy::bytes()?;
        let mut packet = Zeroizing::new([0; 4096]);
        let len = request::encode(config, credentials, entropy, &mut packet);
        let socket = UdpSocket::bind(SocketAddrV4::new(crate::lab::profile::WAN_IP.into(), 0))?;
        socket.connect(config.endpoint)?;
        Ok(exchange(&socket, &packet[..len], config, Instant::now))
    };
    operation().unwrap_or(Decision {
        outcome: Outcome::Unavailable,
        elapsed_ms: started.elapsed().as_millis().min(u128::from(u32::MAX)) as u32,
        invalid: 0,
        sends: 0,
    })
}
trait Channel {
    fn send(&self, bytes: &[u8]) -> io::Result<usize>;
    fn receive(&self, bytes: &mut [u8], timeout: Duration) -> io::Result<usize>;
}
impl Channel for UdpSocket {
    fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        UdpSocket::send(self, bytes)
    }
    fn receive(&self, bytes: &mut [u8], timeout: Duration) -> io::Result<usize> {
        self.set_read_timeout(Some(timeout))?;
        self.recv(bytes)
    }
}
fn exchange(
    socket: &impl Channel,
    request: &[u8],
    config: &Config,
    now: impl Fn() -> Instant,
) -> Decision {
    let started = now();
    let deadline = started + config.timeout;
    let retry = started + config.timeout / 2;
    let mut invalid = 0u16;
    let mut sends = 0;
    let mut response = [0; 4097];
    let outcome = loop {
        let time = now();
        if time >= deadline {
            break if invalid > 0 {
                Outcome::Invalid
            } else {
                Outcome::Timeout
            };
        }
        if sends == 0 || (sends == 1 && time >= retry) {
            if socket.send(request).ok() != Some(request.len()) {
                break Outcome::Unavailable;
            }
            sends += 1;
        }
        let until = if sends == 1 { retry } else { deadline };
        let wait = until
            .saturating_duration_since(now())
            .min(Duration::from_millis(150));
        if wait.is_zero() {
            continue;
        }
        match socket.receive(&mut response, wait) {
            Ok(len) => match response::decode(&response[..len], request, &config.secret) {
                Outcome::Invalid => invalid = invalid.saturating_add(1),
                outcome => break outcome,
            },
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(_) => break Outcome::Unavailable,
        }
    };
    Decision {
        outcome,
        elapsed_ms: now()
            .saturating_duration_since(started)
            .as_millis()
            .min(u128::from(u32::MAX)) as u32,
        invalid,
        sends,
    }
}
#[cfg(test)]
mod tests;
