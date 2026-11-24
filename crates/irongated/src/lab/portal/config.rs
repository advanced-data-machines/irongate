//! Explicit lab-only listener/origin contract. No inferred HTTP downgrade.
use std::{io, net::SocketAddrV4, path::Path, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) struct Endpoint {
    pub origin: String,
    pub authority: String,
    pub address: SocketAddrV4,
    pub secure: bool,
}
impl Endpoint {
    pub fn parse(origin: &str) -> io::Result<Self> {
        let uri: http::Uri = origin
            .parse()
            .map_err(|_| invalid("invalid portal origin"))?;
        let secure = match uri.scheme_str() {
            Some("https") => true,
            Some("http") => false,
            _ => return Err(invalid("portal scheme must be http or https")),
        };
        let authority = uri
            .authority()
            .ok_or_else(|| invalid("portal authority missing"))?;
        let port = authority
            .port_u16()
            .ok_or_else(|| invalid("explicit portal port required"))?;
        if authority.as_str().contains('@')
            || authority.host() != "10.50.10.1"
            || port != if secure { 8443 } else { 8080 }
            || uri.path_and_query().is_some_and(|v| v.as_str() != "/")
            || origin != format!("{}://{authority}", if secure { "https" } else { "http" })
        {
            return Err(invalid("origin must be the exact owned lab endpoint"));
        }
        Ok(Self {
            origin: origin.to_owned(),
            authority: authority.to_string(),
            address: SocketAddrV4::new(crate::lab::profile::GATEWAY_IP.into(), port),
            secure,
        })
    }
    #[cfg(test)]
    pub fn http() -> Self {
        Self::parse("http://10.50.10.1:8080").unwrap()
    }
}

pub(crate) struct Settings {
    pub endpoint: Endpoint,
    pub tls: Option<Arc<rustls::ServerConfig>>,
}
impl Settings {
    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = super::tls_config::read_bounded(path, 4096, false)?;
        Self::parse(std::str::from_utf8(&bytes).map_err(|_| invalid("config is not UTF-8"))?)
    }
    pub fn parse(text: &str) -> io::Result<Self> {
        let parsed = toml::de::DeTable::parse(text).map_err(|_| invalid("invalid portal TOML"))?;
        let table = parsed.get_ref();
        if table.keys().any(|k| {
            !["mode", "origin", "certificate_der", "private_key_der"]
                .contains(&k.get_ref().as_ref())
        }) {
            return Err(invalid("unknown portal setting"));
        }
        let string = |key| {
            table
                .get(key)
                .and_then(|v| v.get_ref().as_str())
                .ok_or_else(|| invalid("missing or invalid portal setting"))
        };
        let endpoint = Endpoint::parse(string("origin")?)?;
        let tls = match (string("mode")?, endpoint.secure) {
            ("https", true) => Some(super::tls_config::load(
                Path::new(string("certificate_der")?),
                Path::new(string("private_key_der")?),
            )?),
            ("http-lab", false)
                if !table.contains_key("certificate_der")
                    && !table.contains_key("private_key_der") =>
            {
                None
            }
            _ => return Err(invalid("portal mode/origin/credentials mismatch")),
        };
        Ok(Self { endpoint, tls })
    }
}
pub(super) fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
