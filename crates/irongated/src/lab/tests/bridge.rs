//! Real packet bridge and policy cell tested without sockets or privileges.

use super::common;
use crate::lab::{profile::Side, state::State};
use irongate_core::{lease, session};
use std::{
    fs::File,
    io::ErrorKind,
    os::{fd::AsFd, unix::net::UnixDatagram},
};

#[test]
fn wire_dora_then_consent_logout_and_expiry_gate_actual_output_submission() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    let mut output = [0; 2048];
    for hex in [
        include_str!("../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        assert!(
            state
                .packet(Side::Down, &common::decode(hex), &mut output)
                .is_some()
        );
    }
    assert_eq!(state.control(b"status")["phase"], "Leased");
    let tcp = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/routed-input.hex"
    ));
    assert!(state.packet(Side::Down, &tcp, &mut output).is_none());
    assert_eq!(state.control(b"consent")["allowed"], true);
    assert!(matches!(
        state.packet(Side::Down, &tcp, &mut output),
        Some((Side::Up, 54))
    ));
    let mut spoof = tcp.clone();
    spoof[6] = 4;
    assert!(state.packet(Side::Down, &spoof, &mut output).is_none());
    assert_eq!(state.control(b"logout")["allowed"], false);
    assert!(state.packet(Side::Down, &tcp, &mut output).is_none());
    state.control(b"consent");
    state.advance(5).unwrap();
    assert!(!state.allowed());
    assert!(state.applied.is_none());
    assert!(state.packet(Side::Down, &tcp, &mut output).is_none());
    assert_eq!(state.counters.applies, 2);
    assert_eq!(state.counters.removals, 2);
}

#[test]
fn nonblocking_datagram_dispatch_only_submits_authorized_route_and_counts_send_failures() {
    let (down, client) = UnixDatagram::pair().unwrap();
    let (up, origin) = UnixDatagram::pair().unwrap();
    for socket in [&down, &client, &up, &origin] {
        socket.set_nonblocking(true).unwrap();
    }
    let down = File::from(down.as_fd().try_clone_to_owned().unwrap());
    let up = File::from(up.as_fd().try_clone_to_owned().unwrap());
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    let mut input = [0; 2049];
    let mut output = [0; 2048];
    for hex in [
        include_str!("../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        client.send(&common::decode(hex)).unwrap();
        crate::lab::io::packet_budget(
            &mut state,
            Side::Down,
            [&down, &up],
            &mut input,
            &mut output,
        )
        .unwrap();
        assert!(client.recv(&mut input).unwrap() >= 342);
    }
    let tcp = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/routed-input.hex"
    ));
    client.send(&tcp).unwrap();
    crate::lab::io::packet_budget(
        &mut state,
        Side::Down,
        [&down, &up],
        &mut input,
        &mut output,
    )
    .unwrap();
    assert_eq!(
        origin.recv(&mut input).unwrap_err().kind(),
        ErrorKind::WouldBlock
    );
    state.control(b"consent");
    client.send(&tcp).unwrap();
    crate::lab::io::packet_budget(
        &mut state,
        Side::Down,
        [&down, &up],
        &mut input,
        &mut output,
    )
    .unwrap();
    assert_eq!(origin.recv(&mut input).unwrap(), 54);
    assert_eq!(
        &input[..54],
        &common::decode(include_str!(
            "../../../../irongate-proto/tests/fixtures/routed-output.hex"
        ))
    );
    assert_eq!(state.counters.forwarded, 1);
    assert_eq!(state.counters.forwarded_ip_bytes, 40);
    drop(origin);
    client.send(&tcp).unwrap();
    crate::lab::io::packet_budget(
        &mut state,
        Side::Down,
        [&down, &up],
        &mut input,
        &mut output,
    )
    .unwrap();
    assert_eq!(state.counters.tx_errors, 1);
    assert_eq!(state.counters.forwarded, 1);
}

#[test]
fn bounded_control_dispatch_replies_and_records_an_unreplyable_sender() {
    let (server, client) = UnixDatagram::pair().unwrap();
    server.set_nonblocking(true).unwrap();
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    client.send(b"status").unwrap();
    // Paired datagrams have unnamed peer addresses; send_to_addr must fail
    // visibly rather than silently losing diagnostic replies.
    crate::lab::io::control_budget(&mut state, &server).unwrap();
    assert_eq!(state.counters.control_errors, 1);
    assert!(!state.allowed());
}

#[test]
fn live_bridge_keeps_naks_silence_renewal_release_and_quarantine_distinct() {
    let mut leases = [lease::Slot::EMPTY];
    let mut sessions = [session::Slot::EMPTY];
    let mut state = State::new(&mut leases, &mut sessions);
    let mut output = [0; 2048];
    let discover = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-discover.hex"
    ));
    let selecting = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-selecting.hex"
    ));
    assert_eq!(state.control(b"consent")["ok"], false);
    assert_eq!(state.control(b"not-a-command")["ok"], false);
    assert!(state.packet(Side::Down, &selecting, &mut output).is_some());
    assert_eq!(state.counters.naks, 1);
    state.packet(Side::Down, &discover, &mut output).unwrap();
    let other = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-select-other.hex"
    ));
    assert!(state.packet(Side::Down, &other, &mut output).is_none());
    state.packet(Side::Down, &discover, &mut output).unwrap();
    state.packet(Side::Down, &selecting, &mut output).unwrap();
    for hex in [
        include_str!("../../../../irongate-proto/tests/fixtures/lab-renew.hex"),
        include_str!("../../../../irongate-proto/tests/fixtures/lab-reboot.hex"),
    ] {
        assert!(
            state
                .packet(Side::Down, &common::decode(hex), &mut output)
                .is_some()
        );
        assert!(!state.allowed());
    }
    assert_eq!(state.control(b"consent")["allowed"], true);
    let release = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-release.hex"
    ));
    assert!(state.packet(Side::Down, &release, &mut output).is_none());
    assert!(!state.allowed());
    state.packet(Side::Down, &discover, &mut output).unwrap();
    state.packet(Side::Down, &selecting, &mut output).unwrap(); // New lease/session generation.
    assert_eq!(state.control(b"status")["phase"], "Leased");
    let decline = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-decline.hex"
    ));
    assert!(state.packet(Side::Down, &decline, &mut output).is_none());
    assert!(state.packet(Side::Down, &discover, &mut output).is_none()); // Quarantined, not available.
    state.advance(30).unwrap();
    let identified = common::decode(include_str!(
        "../../../../irongate-proto/tests/fixtures/lab-discover-client-id.hex"
    ));
    assert!(state.packet(Side::Down, &identified, &mut output).is_some());
    assert!(!state.allowed());
}
