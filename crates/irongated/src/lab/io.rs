//! Guest-only inherited AF_PACKET descriptors and bounded readiness dispatch.

use super::{owner, profile::Side, state::State, transport::Transport};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::unix::net::UnixDatagram,
};

pub fn run(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    guard(run_id)?;
    let mut transport = Transport::inherited()?;
    let (portal, worker) = super::portal::Bridge::pair()?;
    let provider = super::portal::start(worker, run_id)?;
    transport.portal = Some(portal);
    let mut lease_slots = [irongate_core::lease::Slot::EMPTY];
    let mut session_slots = [irongate_core::session::Slot::EMPTY];
    let mut state = State::new(&mut lease_slots, &mut session_slots);
    state.provider = provider;
    eprintln!("IronGate lab ready; run={run_id}; upstream initially denied");
    owner::serve(&mut state, &transport)
}

// Bounded controls precede data so logout cannot be starved by queued packets.
pub(super) fn control_budget(
    state: &mut State<'_>,
    control: &UnixDatagram,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..8 {
        let mut command = [0; 64];
        match control.recv_from(&mut command) {
            Ok((length, address)) => {
                let reply = serde_json::to_vec(&state.control(&command[..length]))?;
                if control.send_to_addr(&reply, &address).is_err() {
                    state.counters.control_errors = state.counters.control_errors.saturating_add(1);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub(super) fn packet_budget(
    state: &mut State<'_>,
    side: Side,
    sockets: [&File; 2],
    input: &mut [u8],
    output: &mut [u8],
) -> io::Result<()> {
    for _ in 0..64 {
        let mut source = if side == Side::Down {
            sockets[0]
        } else {
            sockets[1]
        };
        let length = match source.read(input) {
            Ok(length) => length,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if let Some((destination, used)) = state.packet(side, &input[..length], output) {
            let mut target = if destination == Side::Down {
                sockets[0]
            } else {
                sockets[1]
            };
            match target.write(&output[..used]) {
                Ok(sent) if sent == used => {
                    if side != destination {
                        state.counters.forwarded = state.counters.forwarded.saturating_add(1);
                        state.counters.forwarded_ip_bytes = state
                            .counters
                            .forwarded_ip_bytes
                            .saturating_add((used - 14) as u64);
                    }
                }
                _ => state.counters.tx_errors = state.counters.tx_errors.saturating_add(1),
            }
        }
    }
    Ok(())
}

fn guard(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    if run_id.len() != 32 || !run_id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid lab run identity".into());
    }
    let marker = std::fs::read_to_string("/run/irongate-packet-lab/id")?;
    let vendor = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor")?;
    if marker.trim() != run_id
        || vendor.trim() != "QEMU"
        || std::fs::read_link("/proc/self/ns/net")? == std::fs::read_link("/proc/1/ns/net")?
    {
        return Err("lab is restricted to the owned QEMU network namespace".into());
    }
    Ok(())
}
