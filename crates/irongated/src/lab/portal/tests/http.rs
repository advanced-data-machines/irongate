use crate::lab::{
    portal::{
        bridge::{Envelope, Reply},
        protocol::{self, Parsed},
        server,
    },
    profile,
};
use std::{
    io::{self, Read, Write},
    sync::mpsc,
};

fn parse(bytes: &[u8], entropy: [u8; 32]) -> Result<Parsed, u16> {
    protocol::parse(bytes, entropy, &super::super::config::Endpoint::http())
}
fn get() -> Vec<u8> {
    b"GET / HTTP/1.1\r\nHost: 10.50.10.1:8080\r\n\r\n".to_vec()
}
fn post(extra: &str, body: &str) -> Vec<u8> {
    format!("POST /consent HTTP/1.1\r\nHost: 10.50.10.1:8080\r\nOrigin: http://10.50.10.1:8080\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n{extra}\r\n{body}",body.len()).into_bytes()
}

#[test]
fn complete_framing_and_each_truncated_get_are_distinct() {
    let request = get();
    for end in 0..request.len() {
        assert!(matches!(
            parse(&request[..end], [1; 32]),
            Ok(Parsed::Incomplete)
        ));
    }
    assert!(matches!(
        parse(&request, [1; 32]),
        Ok(Parsed::Ready { html: true, .. })
    ));
    let body = format!("token={}", protocol::encode([1; 32]));
    assert!(matches!(
        parse(&post("", &body), [2; 32]),
        Ok(Parsed::Ready { html: false, .. })
    ));
    assert!(protocol::decode(b"xyz").is_none());
    assert!(protocol::decode(&[b'g'; 64]).is_none());
}

#[test]
fn smuggling_cross_origin_and_unbounded_requests_fail_closed() {
    let body = format!("token={}", protocol::encode([1; 32]));
    for bytes in [
        post("Content-Length: 70\r\n", &body),
        post("Transfer-Encoding: chunked\r\n", &body),
        [get(), get()].concat(),
        post("Host: malicious.invalid\r\n", &body),
    ] {
        assert!(parse(&bytes, [1; 32]).is_err());
    }
    let changed = String::from_utf8(post("", &body)).unwrap().replace(
        "Origin: http://10.50.10.1:8080",
        "Origin: http://attacker.invalid",
    );
    assert!(matches!(parse(changed.as_bytes(), [1; 32]), Err(403)));
    let opaque = changed.replace("http://attacker.invalid", "null");
    assert!(matches!(parse(opaque.as_bytes(), [1; 32]), Err(403)));
    assert!(matches!(
        parse(&post("", &"x".repeat(129)), [1; 32]),
        Err(413)
    ));
    assert!(parse(&vec![0; protocol::MAX_REQUEST + 1], [1; 32]).is_err());
}

struct Stream {
    input: io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}
impl super::super::transport::RequestStream for Stream {}
impl Read for Stream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.input.read(out)
    }
}
impl Write for Stream {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn worker_only_sends_typed_requests_and_has_a_bounded_busy_response() {
    let (sender, receiver) = mpsc::sync_channel::<Envelope>(1);
    let mut stream = Stream {
        input: io::Cursor::new(get()),
        output: vec![],
    };
    server::handle(
        &mut stream,
        profile::CLIENT_IP.into(),
        &sender,
        || {
            let envelope = receiver.try_recv().unwrap();
            assert_eq!(envelope.peer.octets(), profile::CLIENT_IP);
            envelope
                .reply
                .try_send(Reply::Redirect)
                .unwrap_or_else(|_| panic!());
        },
        &super::super::config::Settings {
            endpoint: super::super::config::Endpoint::http(),
            tls: None,
            radius: None,
            accounting: None,
        },
        &super::super::metrics::Metrics::default(),
    )
    .unwrap();
    let response = String::from_utf8(stream.output).unwrap();
    assert!(response.starts_with("HTTP/1.1 303"));
    assert!(response.contains("Cache-Control: no-store"));
    assert!(response.contains("Referrer-Policy: same-origin"));
    let (full, _receiver) = mpsc::sync_channel(0);
    let mut stream = Stream {
        input: io::Cursor::new(get()),
        output: vec![],
    };
    server::handle(
        &mut stream,
        profile::CLIENT_IP.into(),
        &full,
        || panic!("full queue must not wake owner"),
        &super::super::config::Settings {
            endpoint: super::super::config::Endpoint::http(),
            tls: None,
            radius: None,
            accounting: None,
        },
        &super::super::metrics::Metrics::default(),
    )
    .unwrap();
    assert!(stream.output.starts_with(b"HTTP/1.1 503"));
}
