//! Explicit single endpoint and file-based shared secret; no implicit fallback.
use std::{io, net::SocketAddrV4, path::Path, time::Duration};
use zeroize::Zeroizing;
pub(crate) struct Config {
    pub endpoint: SocketAddrV4,
    pub nas_identifier: String,
    pub secret: Zeroizing<Vec<u8>>,
    pub timeout: Duration,
}
impl Config {
    pub fn load(table: &toml::de::DeTable<'_>) -> io::Result<Self> {
        let text = |k| {
            table
                .get(k)
                .and_then(|v| v.get_ref().as_str())
                .ok_or_else(|| io::Error::other("missing RADIUS setting"))
        };
        let endpoint: SocketAddrV4 = text("radius_endpoint")?
            .parse()
            .map_err(|_| io::Error::other("invalid RADIUS endpoint"))?;
        if endpoint.ip().octets() != crate::lab::profile::UPSTREAM_IP || endpoint.port() != 1812 {
            return Err(io::Error::other(
                "RADIUS endpoint must be the owned lab backend",
            ));
        }
        let nas = text("nas_identifier")?;
        if nas.is_empty()
            || nas.len() > 64
            || !nas
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        {
            return Err(io::Error::other("invalid NAS identifier"));
        }
        let bytes = super::super::portal::tls_config::read_bounded(
            Path::new(text("radius_secret_file")?),
            128,
            true,
        )?;
        let mut secret = Zeroizing::new(bytes);
        while secret.last() == Some(&b'\n') || secret.last() == Some(&b'\r') {
            secret.pop();
        }
        if !(16..=64).contains(&secret.len()) || !secret.iter().all(|b| (33..=126).contains(b)) {
            return Err(io::Error::other("invalid RADIUS secret length/encoding"));
        }
        Ok(Self {
            endpoint,
            nas_identifier: nas.to_owned(),
            secret,
            timeout: Duration::from_secs(2),
        })
    }
}
