//! Accounting is explicit, single-sink and restricted to the owned lab address.
use std::{io, net::SocketAddrV4, sync::Arc};
pub(crate) struct Config {
    pub radius: Arc<crate::lab::radius::config::Config>,
    pub endpoint: SocketAddrV4,
    pub interval: u64,
}
impl Config {
    pub fn load(
        table: &toml::de::DeTable<'_>,
        radius: Option<&Arc<crate::lab::radius::config::Config>>,
    ) -> io::Result<Option<Arc<Self>>> {
        let fields = [
            "accounting",
            "accounting_endpoint",
            "accounting_interim_seconds",
        ];
        if !fields.iter().any(|k| table.contains_key(*k)) {
            return Ok(None);
        }
        let text = |k| {
            table
                .get(k)
                .and_then(|v| v.get_ref().as_str())
                .ok_or_else(|| io::Error::other("missing accounting setting"))
        };
        if text("accounting")? != "radius" {
            return Err(io::Error::other("unsupported accounting provider"));
        }
        let radius =
            radius.ok_or_else(|| io::Error::other("accounting requires RADIUS authentication"))?;
        let endpoint: SocketAddrV4 = text("accounting_endpoint")?
            .parse()
            .map_err(|_| io::Error::other("invalid accounting endpoint"))?;
        if endpoint.ip().octets() != crate::lab::profile::UPSTREAM_IP || endpoint.port() != 1813 {
            return Err(io::Error::other(
                "accounting endpoint must be the owned lab sink",
            ));
        }
        let interval = table
            .get("accounting_interim_seconds")
            .and_then(|v| v.get_ref().as_integer())
            .and_then(|i| u64::from_str_radix(i.as_str(), i.radix()).ok())
            .ok_or_else(|| io::Error::other("accounting interval required"))?;
        if !(1..=60).contains(&interval) {
            return Err(io::Error::other(
                "lab accounting interval must be 1..60 seconds",
            ));
        }
        Ok(Some(Arc::new(Self {
            radius: Arc::clone(radius),
            endpoint,
            interval,
        })))
    }
}
