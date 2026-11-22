//! Explicit one-client lab profile; these are not production TOML defaults.

use irongate_core::{lease, session};
use irongate_proto::{arp, routed};

pub const CLIENT_IP: [u8; 4] = [10, 50, 10, 100];
pub const GATEWAY_IP: [u8; 4] = [10, 50, 10, 1];
pub const UPSTREAM_IP: [u8; 4] = [192, 0, 2, 80];
pub const WAN_IP: [u8; 4] = [192, 0, 2, 1];
pub const CLIENT_MAC: [u8; 6] = [2, 0, 0, 0, 0, 2];
pub const DOWN_MAC: [u8; 6] = [2, 0, 0, 0, 0, 1];
pub const WAN_MAC: [u8; 6] = [2, 0, 0, 0, 1, 1];
pub const UPSTREAM_MAC: [u8; 6] = [2, 0, 0, 0, 1, 2];
pub const CONTROL: &str = "/run/irongate-packet-lab/control.sock";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Down,
    Up,
}

impl Side {
    pub fn arp(self) -> arp::Endpoint {
        match self {
            Self::Down => arp::Endpoint {
                mac: DOWN_MAC,
                ip: GATEWAY_IP,
                peer_mac: CLIENT_MAC,
                peer_ip: CLIENT_IP,
            },
            Self::Up => arp::Endpoint {
                mac: WAN_MAC,
                ip: WAN_IP,
                peer_mac: UPSTREAM_MAC,
                peer_ip: UPSTREAM_IP,
            },
        }
    }
    pub fn route(self) -> routed::Route {
        match self {
            Self::Down => routed::Route {
                source_mac: CLIENT_MAC,
                gateway_mac: DOWN_MAC,
                source_ip: CLIENT_IP,
                destination_ip: UPSTREAM_IP,
                output_mac: WAN_MAC,
                next_hop_mac: UPSTREAM_MAC,
            },
            Self::Up => routed::Route {
                source_mac: UPSTREAM_MAC,
                gateway_mac: WAN_MAC,
                source_ip: UPSTREAM_IP,
                destination_ip: CLIENT_IP,
                output_mac: DOWN_MAC,
                next_hop_mac: CLIENT_MAC,
            },
        }
    }
    pub fn opposite(self) -> Self {
        if self == Self::Down {
            Self::Up
        } else {
            Self::Down
        }
    }
}

pub fn lease_config() -> lease::PoolConfig {
    lease::PoolConfig {
        network: lease::NetworkId(1),
        epoch: 1,
        server: GATEWAY_IP,
        prefix: 24,
        first: CLIENT_IP,
        last: CLIENT_IP,
        lease_seconds: 60,
        offer_seconds: 10,
        decline_seconds: 30,
    }
}
pub fn session_config() -> session::Config {
    session::Config {
        network: lease::NetworkId(1),
        epoch: 1,
        consent_seconds: 10,
        max_authorized_seconds: 5,
    }
}
