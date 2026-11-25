use super::{Outbox, model::*};
use crate::lab::{profile, state::State, tests::common};
use irongate_core::{lease, session};
mod lifecycle;
mod protocol;
mod scheduling;
fn bound<'a>(leases: &'a mut [lease::Slot], sessions: &'a mut [session::Slot]) -> State<'a> {
    let mut state = State::new(leases, sessions);
    for fixture in [
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-discover.hex"),
        include_str!("../../../../../irongate-proto/tests/fixtures/lab-selecting.hex"),
    ] {
        state
            .packet(
                profile::Side::Down,
                &common::decode(fixture),
                &mut [0; 2048],
            )
            .unwrap();
    }
    state
}
fn identity() -> UserName {
    UserName::new(b"lab-user").unwrap()
}
fn ack(dispatch: Dispatch) -> Receipt {
    Receipt {
        token: dispatch.token,
        outcome: Outcome::Ack,
        invalid: 0,
    }
}
fn handles() -> (session::Handle, lease::Handle) {
    let mut l = [lease::Slot::EMPTY];
    let mut s = [session::Slot::EMPTY];
    let state = bound(&mut l, &mut s);
    let h = state.active.unwrap();
    (h, state.sessions.view(h).unwrap().lease)
}
