//! Real readiness/dispatch with local socket pairs, no IP/raw host sockets.

use super::common;
use crate::lab::{
    owner::{self, Buffers},
    state::State,
    transport::Transport,
};
use irongate_core::{lease, session};
use std::{
    fs::File,
    io::ErrorKind,
    os::{fd::AsFd, unix::net::UnixDatagram},
};

fn pair() -> (File, UnixDatagram) {
    let (owner, peer) = UnixDatagram::pair().unwrap();
    owner.set_nonblocking(true).unwrap();
    peer.set_nonblocking(true).unwrap();
    (
        File::from(owner.as_fd().try_clone_to_owned().unwrap()),
        peer,
    )
}

#[test]
fn event_turn_prioritizes_logout_over_queued_transit_and_advances_real_state() {
    let (down, client) = pair();
    let (up, upstream) = pair();
    let (control, admin) = UnixDatagram::pair().unwrap();
    let transport = Transport::new(down, up, control).unwrap();
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    let mut buffers = Buffers::default();
    for hex in [
        include_str!("../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        client.send(&common::decode(hex)).unwrap();
        owner::turn(&mut state, &transport, &mut buffers, || 0).unwrap();
        client.recv(&mut buffers.input).unwrap();
    }
    let frame = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/routed-input.hex"
    ));
    state.control(b"consent");
    client.send(&frame).unwrap();
    owner::turn(&mut state, &transport, &mut buffers, || 1).unwrap();
    assert_eq!(upstream.recv(&mut buffers.input).unwrap(), 54);
    client.send(&frame).unwrap();
    admin.send(b"logout").unwrap();
    owner::turn(&mut state, &transport, &mut buffers, || 2).unwrap();
    assert!(!state.allowed());
    assert_eq!(
        upstream.recv(&mut buffers.input).unwrap_err().kind(),
        ErrorKind::WouldBlock
    );
    assert_eq!(state.sessions.now(), 2);
    assert_eq!(state.counters.removals, 1);
}

#[test]
fn transport_rejects_regular_packet_descriptors_and_owner_propagates_clock_errors() {
    let (_, peer) = pair();
    let (control, _) = UnixDatagram::pair().unwrap();
    let invalid = Transport::new(
        File::open("/dev/null").unwrap(),
        File::from(peer.as_fd().try_clone_to_owned().unwrap()),
        control,
    );
    assert!(matches!(invalid, Err(error) if error.kind() == ErrorKind::InvalidInput));
    let (down, _client) = pair();
    let (up, _upstream) = pair();
    let (control, _admin) = UnixDatagram::pair().unwrap();
    let transport = Transport::new(down, up, control).unwrap();
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    state.advance(10).unwrap();
    assert!(owner::turn(&mut state, &transport, &mut Buffers::default(), || 9).is_err());
}
