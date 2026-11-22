//! Guest-only inherited AF_PACKET descriptors and bounded readiness dispatch.

use super::{
    profile::{self, Side},
    state::State,
};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::AsFd,
        unix::{fs::FileTypeExt, net::UnixDatagram},
    },
    time::Instant,
};

pub fn run(run_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    guard(run_id)?;
    // The guest supervisor binds raw sockets, disables offload and nonblocks them.
    // Cloning known standard descriptors needs no unsafe FD construction.
    let down = File::from(io::stdin().as_fd().try_clone_to_owned()?);
    let up = File::from(io::stdout().as_fd().try_clone_to_owned()?);
    if !down.metadata()?.file_type().is_socket() || !up.metadata()?.file_type().is_socket() {
        return Err("lab requires two inherited packet sockets".into());
    }
    let control = UnixDatagram::bind(profile::CONTROL)?;
    control.set_nonblocking(true)?;
    let mut lease_slots = [irongate_core::lease::Slot::EMPTY];
    let mut session_slots = [irongate_core::session::Slot::EMPTY];
    let mut state = State::new(&mut lease_slots, &mut session_slots);
    let start = Instant::now();
    let mut input = [0; irongate_proto::MAX_FRAME_BYTES + 1];
    let mut output = [0; irongate_proto::MAX_FRAME_BYTES];
    eprintln!("IronGate lab ready; run={run_id}; upstream initially denied");
    loop {
        state.advance(start.elapsed().as_secs())?;
        wait(&down, &up, &control)?;
        state.advance(start.elapsed().as_secs())?;
        control_budget(&mut state, &control)?;
        for side in [Side::Down, Side::Up] {
            packet_budget(&mut state, side, [&down, &up], &mut input, &mut output)?;
        }
    }
}

fn wait(down: &File, up: &File, control: &UnixDatagram) -> Result<(), Box<dyn std::error::Error>> {
    let mut descriptors = [
        PollFd::new(down, PollFlags::IN),
        PollFd::new(up, PollFlags::IN),
        PollFd::new(control, PollFlags::IN),
    ];
    let timeout = Timespec {
        tv_sec: 0,
        tv_nsec: 100_000_000,
    };
    match poll(&mut descriptors, Some(&timeout)) {
        Ok(_) | Err(rustix::io::Errno::INTR) => (),
        Err(error) => return Err(error.into()),
    }
    if descriptors.iter().any(|fd| {
        fd.revents()
            .intersects(PollFlags::ERR | PollFlags::HUP | PollFlags::NVAL)
    }) {
        return Err("lab descriptor failed; forwarding stopped".into());
    }
    Ok(())
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
