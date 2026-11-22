//! Assertion-based synthetic fixture adapter, not a daemon/network API.

use irongate_core::lease::{
    ClientIdentity, ClientKey, Command, NetworkId, Outcome, PoolConfig, RequestMode, Table,
};
use irongate_proto::{
    dhcp::{self, ClientMessage, Offer, Server},
    parse_request,
};

pub const NETWORK: NetworkId = NetworkId(1);

pub fn config() -> PoolConfig {
    PoolConfig {
        network: NETWORK,
        epoch: 1,
        server: [192, 0, 2, 1],
        prefix: 24,
        first: [192, 0, 2, 100],
        last: [192, 0, 2, 103],
        lease_seconds: 3600,
        offer_seconds: 10,
        decline_seconds: 30,
    }
}

/// All inputs are known fixtures. A live adapter must replace these assertions
/// with bounded diagnostics, validate its config/output capacity before mutation,
/// and consume expiry/release effects before allowing address reuse.
pub fn exchange(table: &mut Table<'_>, bytes: &[u8], output: &mut [u8]) -> (Outcome, usize) {
    let request = parse_request(bytes).expect("valid synthetic frame");
    let message = request
        .lease_message()
        .expect("supported synthetic command");
    let identity = match request.dhcp().option(61) {
        Some(id) => ClientIdentity::option_61(id),
        None => ClientIdentity::ethernet(request.dhcp().client_mac()),
    }
    .expect("valid synthetic identity");
    // In production this comes from the trusted receiving interface/VLAN map.
    let client = ClientKey {
        network: NETWORK,
        identity,
    };
    let xid = request.dhcp().transaction_id();
    let command = match message {
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
    let outcome = table.apply(command).expect("fixture pool has capacity");
    let server = Server {
        mac: [2, 0, 0, 0, 0, 1],
        ip: config().server,
    };
    let length = match outcome {
        Outcome::Offer(grant) | Outcome::Ack(grant) => {
            let reply = Offer {
                server_mac: server.mac,
                server_ip: server.ip,
                offered_ip: grant.address,
                subnet_mask: [255, 255, 255, 0],
                router: server.ip,
                dns: [192, 0, 2, 53],
                lease_seconds: grant.lease_seconds,
            };
            if matches!(outcome, Outcome::Offer(_)) {
                dhcp::encode_offer(&request, &reply, output)
            } else {
                dhcp::encode_ack(&request, &reply, output)
            }
            .expect("fixture reply fits")
        }
        Outcome::Nak(_) => dhcp::encode_nak(&request, server, output).expect("fixture NAK fits"),
        Outcome::Ignore(_) | Outcome::Released(_) | Outcome::Declined(_) => 0,
    };
    (outcome, length)
}
