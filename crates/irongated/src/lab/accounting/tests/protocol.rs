use super::*;
use crate::lab::{
    accounting::{config::Config, wire, worker},
    radius::{client::Channel, config::Config as Radius},
};
use std::{
    cell::{Cell, RefCell},
    io,
    sync::Arc,
    time::{Duration, Instant},
};
fn config() -> Config {
    Config {
        radius: Arc::new(Radius {
            endpoint: "192.0.2.80:1812".parse().unwrap(),
            nas_identifier: "irongate-lab".into(),
            secret: zeroize::Zeroizing::new(b"test-accounting-secret".to_vec()),
            timeout: Duration::from_secs(2),
        }),
        endpoint: "192.0.2.80:1813".parse().unwrap(),
        interval: 2,
    }
}
#[test]
fn independent_vectors_cover_authenticators_directions_and_gigawords() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([1; 16], 2);
    q.reserve(h, l, identity(), 0);
    q.activate(h, l, 0);
    let start = q.next(0).unwrap();
    q.complete(ack(start), 0);
    q.submitted(h, l, true, (3u64 << 32) + 17);
    q.submitted(h, l, false, (2u64 << 32) + 43);
    q.stop(h, l, 9, 1);
    let dispatch = q.next(12).unwrap();
    let mut packet = [0; 512];
    let len = wire::encode(&config(), dispatch, 73, &mut packet);
    let vectors: serde_json::Value = serde_json::from_str(include_str!("vectors.json")).unwrap();
    assert_eq!(
        packet[..len],
        common::decode(vectors["request"].as_str().unwrap())
    );
    for name in ["wrong_id", "wrong_code", "wrong_length"] {
        let packet_wrong = common::decode(vectors[name].as_str().unwrap());
        assert!(!wire::acknowledged(
            &packet_wrong,
            &packet[..len],
            b"test-accounting-secret"
        ));
    }
    let response = common::decode(vectors["response"].as_str().unwrap());
    assert!(wire::acknowledged(
        &response,
        &packet[..len],
        b"test-accounting-secret"
    ));
    for i in 0..response.len() {
        let mut bad = response.clone();
        bad[i] ^= 1;
        assert!(!wire::acknowledged(
            &bad,
            &packet[..len],
            b"test-accounting-secret"
        ));
    }
    assert!(!wire::acknowledged(
        &response,
        &packet[..len],
        b"wrong-secret"
    ));
    assert!(!wire::acknowledged(
        &response[..19],
        &packet[..len],
        b"test-accounting-secret"
    ));
    let mut bad = response.clone();
    bad.push(0);
    assert!(!wire::acknowledged(
        &bad,
        &packet[..len],
        b"test-accounting-secret"
    ));
    assert!(!wire::acknowledged(
        &response,
        &[],
        b"test-accounting-secret"
    ));
}
struct Fake {
    time: Cell<Instant>,
    replies: RefCell<Vec<Vec<u8>>>,
    sends: Cell<u8>,
    fail: bool,
}
impl Channel for Fake {
    fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.sends.set(self.sends.get() + 1);
        if self.fail {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(bytes.len())
        }
    }
    fn receive(&self, bytes: &mut [u8], timeout: Duration) -> io::Result<usize> {
        self.time.set(self.time.get() + timeout);
        if let Some(reply) = self.replies.borrow_mut().pop() {
            bytes[..reply.len()].copy_from_slice(&reply);
            Ok(reply.len())
        } else {
            Err(io::ErrorKind::TimedOut.into())
        }
    }
}
#[test]
fn one_send_has_bounded_deadline_and_invalid_replies_never_acknowledge() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([0; 16], 2);
    q.reserve(h, l, identity(), 0);
    q.activate(h, l, 0);
    let dispatch = q.next(0).unwrap();
    let mut packet = [0; 512];
    let len = wire::encode(&config(), dispatch, 1, &mut packet);
    for (replies, fail, expected) in [
        (vec![], false, Outcome::Timeout),
        (vec![vec![0; 20]], false, Outcome::Invalid),
        (vec![], true, Outcome::Unavailable),
    ] {
        let started = Instant::now();
        let fake = Fake {
            time: Cell::new(started),
            replies: RefCell::new(replies),
            sends: Cell::new(0),
            fail,
        };
        let receipt = worker::exchange(
            &fake,
            &packet[..len],
            &config().radius.secret,
            dispatch,
            || fake.time.get(),
        );
        assert_eq!(receipt.outcome, expected);
        assert_eq!(fake.sends.get(), 1);
        assert!(fake.time.get() <= started + Duration::from_secs(1));
    }
}
#[test]
fn explicit_config_rejects_partial_external_or_non_radius_accounting() {
    let radius = config().radius;
    let parse = |text: &str| {
        let table = toml::de::DeTable::parse(text).unwrap();
        Config::load(table.get_ref(), Some(&radius))
    };
    assert!(parse("").unwrap().is_none());
    let valid =
        "accounting='radius'\naccounting_endpoint='192.0.2.80:1813'\naccounting_interim_seconds=2";
    assert_eq!(parse(valid).unwrap().unwrap().interval, 2);
    for text in [
        "accounting='radius'".to_owned(),
        valid.replace("1813", "1812"),
        valid.replace("=2", "=0"),
        valid.replace("=2", "=61"),
        valid.replace("=2", "='2'"),
        valid.replace("192.0.2.80", "127.0.0.1"),
        valid.replace("'radius'", "'off'"),
    ] {
        assert!(parse(&text).is_err());
    }
    let table = toml::de::DeTable::parse(valid).unwrap();
    assert!(Config::load(table.get_ref(), None).is_err());
}

#[test]
fn validated_reply_completes_one_exchange_and_a_wrong_identifier_does_not() {
    let (h, l) = handles();
    let mut q = Outbox::enabled([1; 16], 2);
    q.reserve(h, l, identity(), 0);
    q.activate(h, l, 0);
    let start = q.next(0).unwrap();
    q.complete(ack(start), 0);
    q.submitted(h, l, true, (3u64 << 32) + 17);
    q.submitted(h, l, false, (2u64 << 32) + 43);
    q.stop(h, l, 9, 1);
    let dispatch = q.next(12).unwrap();
    let mut packet = [0; 512];
    let len = wire::encode(&config(), dispatch, 73, &mut packet);
    let vectors: serde_json::Value = serde_json::from_str(include_str!("vectors.json")).unwrap();
    let valid = common::decode(vectors["response"].as_str().unwrap());
    let mut wrong = valid.clone();
    wrong[1] ^= 1;
    let fake = Fake {
        time: Cell::new(Instant::now()),
        replies: RefCell::new(vec![valid, wrong]),
        sends: Cell::new(0),
        fail: false,
    };
    let receipt = worker::exchange(
        &fake,
        &packet[..len],
        &config().radius.secret,
        dispatch,
        || fake.time.get(),
    );
    assert_eq!(receipt.outcome, Outcome::Ack);
    assert_eq!(receipt.invalid, 1);
    assert_eq!(fake.sends.get(), 1);
    q.complete(receipt, 13);
    let status = q.snapshot(13).json(13);
    assert_eq!(status["last_acknowledged_kind"], "stop");
    assert_eq!(
        status["last_acknowledged_octets"]["input"],
        (3u64 << 32) + 17
    );
}
