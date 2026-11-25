//! Nonblocking accounting mailbox work has its own fixed owner budget.
use super::bridge::Bridge;
use crate::lab::{
    accounting::{Receipt, model::Outcome},
    state::State,
};
use std::sync::mpsc;
impl Bridge {
    pub(super) fn accounting_budget(&self, state: &mut State<'_>) {
        for _ in 0..2 {
            match self.accounting_acks.try_recv() {
                Ok(receipt) => state.accounting.complete(receipt, state.sessions.now()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    state.accounting.disconnected();
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        if let Some(dispatch) = state.accounting.next(state.sessions.now())
            && self.accounting_sender.try_send(dispatch).is_err()
        {
            state.accounting.complete(
                Receipt {
                    token: dispatch.token,
                    outcome: Outcome::Unavailable,
                    invalid: 0,
                },
                state.sessions.now(),
            );
        }
    }
}
