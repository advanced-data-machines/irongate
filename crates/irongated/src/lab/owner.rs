//! One deterministic, bounded turn shared by the live loop and socket tests.

use super::{
    io::{control_budget, packet_budget},
    profile::Side,
    state::State,
    transport::Transport,
};
use std::time::Instant;

pub(super) struct Buffers {
    pub input: [u8; irongate_proto::MAX_FRAME_BYTES + 1],
    pub output: [u8; irongate_proto::MAX_FRAME_BYTES],
}

impl Default for Buffers {
    fn default() -> Self {
        Self {
            input: [0; irongate_proto::MAX_FRAME_BYTES + 1],
            output: [0; irongate_proto::MAX_FRAME_BYTES],
        }
    }
}

pub(super) fn turn(
    state: &mut State<'_>,
    transport: &Transport,
    buffers: &mut Buffers,
    clock: impl Fn() -> u64,
) -> Result<(), Box<dyn std::error::Error>> {
    state.advance(clock())?;
    transport.wait()?;
    state.advance(clock())?;
    control_budget(state, &transport.control)?;
    if let Some(portal) = &transport.portal {
        portal.budget(state);
    }
    for side in [Side::Down, Side::Up] {
        packet_budget(
            state,
            side,
            [&transport.down, &transport.up],
            &mut buffers.input,
            &mut buffers.output,
        )?;
    }
    Ok(())
}

pub(super) fn serve(
    state: &mut State<'_>,
    transport: &Transport,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffers = Buffers::default();
    let start = Instant::now();
    loop {
        turn(state, transport, &mut buffers, || start.elapsed().as_secs())?;
    }
}
