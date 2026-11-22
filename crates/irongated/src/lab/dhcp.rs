//! Narrow bridge between borrowed DHCP commands and caller-owned live state.

use super::{profile, state::State};
use irongate_core::lease::{self, Command, RequestMode};
use irongate_proto::{
    RequestFrame,
    dhcp::{self, ClientMessage},
};

impl State<'_> {
    pub fn dhcp(&mut self, request: &RequestFrame<'_>, output: &mut [u8]) -> Option<usize> {
        if ![profile::GATEWAY_IP, [255; 4]].contains(&request.ip().destination)
            || ![profile::DOWN_MAC, [255; 6]].contains(&request.ethernet().destination)
        {
            return None;
        }
        let identity = match request.dhcp().option(61) {
            Some(id) => lease::ClientIdentity::option_61(id),
            None => lease::ClientIdentity::ethernet(request.dhcp().client_mac()),
        }
        .ok()?;
        let client = lease::ClientKey {
            network: lease::NetworkId(1),
            identity,
        };
        let xid = request.dhcp().transaction_id();
        let command = match request.lease_message().ok()? {
            ClientMessage::Discover { requested } => Command::Discover {
                client: &client,
                xid,
                requested,
            },
            ClientMessage::Selecting { server, address } => Command::Request {
                client: &client,
                xid,
                mode: RequestMode::Selecting { server, address },
            },
            ClientMessage::InitReboot { address } => Command::Request {
                client: &client,
                xid,
                mode: RequestMode::InitReboot { address },
            },
            ClientMessage::Renew { address } => Command::Request {
                client: &client,
                xid,
                mode: RequestMode::Renew { address },
            },
            ClientMessage::Release { server, address } => Command::Release {
                client: &client,
                server,
                address,
            },
            ClientMessage::Decline { server, address } => Command::Decline {
                client: &client,
                server,
                address,
            },
        };
        let outcome = self.leases.apply(command).ok()?;
        match outcome {
            lease::Outcome::Offer(grant) | lease::Outcome::Ack(grant) => {
                let lease = dhcp::Offer {
                    server_mac: profile::DOWN_MAC,
                    server_ip: profile::GATEWAY_IP,
                    offered_ip: grant.address,
                    subnet_mask: [255, 255, 255, 0],
                    router: profile::GATEWAY_IP,
                    dns: profile::UPSTREAM_IP,
                    lease_seconds: grant.lease_seconds,
                };
                if matches!(outcome, lease::Outcome::Ack(_)) {
                    self.attach(grant.handle)?;
                    self.counters.acks = self.counters.acks.saturating_add(1);
                    dhcp::encode_ack(request, &lease, output).ok()
                } else {
                    self.counters.offers = self.counters.offers.saturating_add(1);
                    dhcp::encode_offer(request, &lease, output).ok()
                }
            }
            lease::Outcome::Nak(_) => {
                self.counters.naks = self.counters.naks.saturating_add(1);
                dhcp::encode_nak(
                    request,
                    dhcp::Server {
                        mac: profile::DOWN_MAC,
                        ip: profile::GATEWAY_IP,
                    },
                    output,
                )
                .ok()
            }
            lease::Outcome::Released(handle) | lease::Outcome::Declined(handle) => {
                self.sessions.lease_lost(handle);
                self.pump().ok()?;
                None
            }
            lease::Outcome::Ignore(_) => None,
        }
    }
}
