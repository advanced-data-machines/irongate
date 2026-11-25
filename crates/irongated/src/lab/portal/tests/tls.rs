//! Real TLS over anonymous local sockets; no host listener or trust changes.
use crate::lab::portal::{
    config::{Endpoint, Settings},
    metrics::Metrics,
    service, tls_config,
    transport::{Secure, Socket},
};
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
const CERT: &[u8] = include_bytes!("fixtures/server.der");
const KEY: &[u8] = include_bytes!("fixtures/server.key.der");
const CA: &[u8] = include_bytes!("fixtures/ca.der");
fn config() -> Arc<rustls::ServerConfig> {
    tls_config::build(CERT.to_vec(), KEY.to_vec()).unwrap()
}
fn client(stream: UnixStream, trusted: bool) -> StreamOwned<ClientConnection, UnixStream> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut roots = RootCertStore::empty();
    if trusted {
        roots.add(CertificateDer::from(CA.to_vec())).unwrap();
    }
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    StreamOwned::new(
        ClientConnection::new(
            Arc::new(config),
            ServerName::IpAddress(std::net::Ipv4Addr::new(10, 50, 10, 1).into()),
        )
        .unwrap(),
        stream,
    )
}
#[test]
fn trusted_https_reuses_typed_http_boundary_and_rejects_wrong_origin() {
    for origin in ["https://10.50.10.1:8443", "http://10.50.10.1:8443"] {
        let (server_sock, client_sock) = UnixStream::pair().unwrap();
        let config = config();
        let thread = std::thread::spawn(move || {
            let socket = Socket::new(server_sock, Instant::now() + Duration::from_secs(2));
            let settings = Settings {
                endpoint: Endpoint::parse("https://10.50.10.1:8443").unwrap(),
                tls: Some(config),
                radius: None,
                accounting: None,
            };
            let (sender, receiver) = mpsc::sync_channel::<crate::lab::portal::bridge::Envelope>(1);
            service::handle(
                socket,
                [10, 50, 10, 100].into(),
                &settings,
                &sender,
                || {
                    receiver
                        .try_recv()
                        .unwrap()
                        .reply
                        .try_send(crate::lab::portal::bridge::Reply::Redirect)
                        .unwrap_or_else(|_| panic!());
                },
                &Metrics::default(),
            )
            .unwrap();
        });
        let mut stream = client(client_sock, true);
        let body = format!("token={}", "11".repeat(32));
        write!(stream,"POST /consent HTTP/1.1\r\nHost: 10.50.10.1:8443\r\nOrigin: {origin}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",body.len()).unwrap();
        stream.flush().unwrap();
        assert_eq!(stream.conn.alpn_protocol(), Some(b"http/1.1".as_slice()));
        let mut bytes = [0; 4096];
        let n = stream.read(&mut bytes).unwrap();
        assert!(bytes[..n].starts_with(if origin.starts_with("https:") {
            b"HTTP/1.1 303"
        } else {
            b"HTTP/1.1 403"
        }));
        thread.join().unwrap();
    }
}
#[test]
fn untrusted_tls_and_expired_total_handshake_deadline_never_reach_http() {
    let (server, client_sock) = UnixStream::pair().unwrap();
    let config = config();
    let thread = std::thread::spawn(move || {
        let mut stream = Secure {
            conn: rustls::ServerConnection::new(config).unwrap(),
            socket: Socket::new(server, Instant::now() + Duration::from_secs(2)),
        };
        assert!(stream.handshake().is_err());
    });
    assert!(
        client(client_sock, false)
            .write_all(b"not authorized")
            .is_err()
    );
    thread.join().unwrap();
    let (server, _client) = UnixStream::pair().unwrap();
    let mut stream = Secure {
        conn: rustls::ServerConnection::new(self::config()).unwrap(),
        socket: Socket::new(server, Instant::now()),
    };
    assert_eq!(
        stream.handshake().unwrap_err().kind(),
        std::io::ErrorKind::TimedOut
    );
}
#[test]
fn explicit_mode_and_origin_reject_downgrades_unknown_settings_and_invalid_credentials() {
    assert!(Settings::parse("mode='http-lab'\norigin='http://10.50.10.1:8080'").is_ok());
    for text in [
        "mode='http-lab'\norigin='https://10.50.10.1:8443'",
        "mode='https'\norigin='http://10.50.10.1:8080'",
        "mode='http-lab'\norigin='http://10.50.10.1:8080'\nunknown=1",
        "mode='http-lab'\norigin='http://10.50.10.1:8080'\nprivate_key_der='ignored'",
    ] {
        assert!(Settings::parse(text).is_err());
    }
    for origin in [
        "https://evil@10.50.10.1:8443",
        "https://10.50.10.1:8443/path",
        "https://10.50.10.1:8443?x=1",
        "https://10.50.10.1",
        "https://10.50.10.1:8080",
        "https://10.50.10.2:8443",
    ] {
        assert!(Endpoint::parse(origin).is_err(), "accepted {origin}");
    }
    assert!(tls_config::build(CERT.to_vec(), vec![0; 32]).is_err());
    assert!(tls_config::build(vec![0; 32], KEY.to_vec()).is_err());
}
