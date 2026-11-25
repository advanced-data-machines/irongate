//! Load bounded DER credentials before listening; one explicit crypto provider.
use super::config::invalid;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::{
    fs::OpenOptions,
    io::{self, Read},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::Arc,
};

pub(crate) fn read_bounded(path: &Path, limit: u64, private: bool) -> io::Result<Vec<u8>> {
    if !path.is_absolute() {
        return Err(invalid("credential/config path must be absolute"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.len() > limit
        || (private && metadata.permissions().mode() & 0o077 != 0)
    {
        return Err(invalid(
            "credential/config type, size or permissions invalid",
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.is_empty() || bytes.len() as u64 > limit {
        return Err(invalid("empty or oversized credential/config"));
    }
    Ok(bytes)
}
pub(super) fn load(certificate: &Path, key: &Path) -> io::Result<Arc<rustls::ServerConfig>> {
    let cert = read_bounded(certificate, 16384, false)?;
    let key = read_bounded(key, 16384, true)?;
    build(cert, key)
}
pub(super) fn build(cert: Vec<u8>, key: Vec<u8>) -> io::Result<Arc<rustls::ServerConfig>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|_| invalid("TLS versions unavailable"))?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(cert)],
            PrivatePkcs8KeyDer::from(key).into(),
        )
        .map_err(|_| invalid("invalid or mismatched TLS credentials"))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config.max_early_data_size = 0;
    config.send_tls13_tickets = 0;
    config.session_storage = Arc::new(rustls::server::NoServerSessionStorage {});
    Ok(Arc::new(config))
}
