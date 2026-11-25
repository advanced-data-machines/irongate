use super::*;
use std::cell::{Cell, RefCell};
struct Silent {
    now: Cell<Instant>,
    sends: RefCell<Vec<Vec<u8>>>,
}
impl Channel for Silent {
    fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.sends.borrow_mut().push(bytes.to_vec());
        Ok(bytes.len())
    }
    fn receive(&self, _: &mut [u8], timeout: Duration) -> io::Result<usize> {
        self.now.set(self.now.get() + timeout);
        Err(io::ErrorKind::TimedOut.into())
    }
}
#[test]
fn one_identical_retransmission_stays_within_the_total_deadline() {
    let channel = Silent {
        now: Cell::new(Instant::now()),
        sends: Default::default(),
    };
    let config = Config {
        endpoint: "192.0.2.80:1812".parse().unwrap(),
        nas_identifier: "test".into(),
        secret: zeroize::Zeroizing::new(b"test-secret-long-enough".to_vec()),
        timeout: Duration::from_secs(2),
    };
    let credentials = Credentials::new("user", "password").unwrap();
    let mut request = [0; 4096];
    let len = request::encode(&config, &credentials, [7; 32], &mut request);
    let decision = exchange(&channel, &request[..len], &config, || channel.now.get());
    assert_eq!(decision.outcome, Outcome::Timeout);
    assert_eq!(decision.sends, 2);
    assert_eq!(decision.elapsed_ms, 2000);
    let sends = channel.sends.borrow();
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[0], sends[1]);
}
