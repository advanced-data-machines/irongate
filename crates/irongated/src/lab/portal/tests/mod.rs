//! Owner, HTTP framing and bounded journal tests without host IP listeners.
mod actions;
use crate::lab::tests::common;
mod http;
mod journal;

use crate::lab::state::State;
use irongate_core::{lease, session};
fn state<'a>(leases: &'a mut [lease::Slot], sessions: &'a mut [session::Slot]) -> State<'a> {
    let mut state = State::new(leases, sessions);
    let mut output = [0; 2048];
    for hex in [
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        state
            .packet(
                crate::lab::profile::Side::Down,
                &common::decode(hex),
                &mut output,
            )
            .unwrap();
    }
    state
}

mod bridge;

mod tls;

mod credentials;

mod login;
