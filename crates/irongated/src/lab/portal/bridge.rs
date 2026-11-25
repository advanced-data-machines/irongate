//! Bounded cross-thread requests and wakeups; all replies are owner observations.

use super::{journal::Record, tokens::Action};
use std::{
    net::Ipv4Addr,
    os::unix::net::UnixDatagram,
    sync::mpsc::{self, Receiver, SyncSender},
};

#[derive(Clone, Copy)]
pub(crate) enum Request {
    View {
        entropy: [u8; 32],
    },
    Status,
    Script,
    Act {
        action: Action,
        token: [u8; 32],
    },
    RadiusResult {
        attempt: irongate_core::session::Attempt,
        decision: crate::lab::radius::Decision,
    },
}

pub(crate) struct Envelope {
    pub peer: Ipv4Addr,
    pub request: Request,
    pub reply: SyncSender<Reply>,
}

#[derive(Clone, Copy)]
pub(crate) struct Snapshot {
    pub phase: &'static str,
    pub provider: irongate_core::session::Provider,
    pub policy_source: Option<irongate_core::session::Provider>,
    pub authentication: crate::lab::radius::health::Health,
    pub transport: Option<super::metrics::Health>,
    pub now: u64,
    pub allowed: bool,
    pub generation: u64,
    pub until: Option<u64>,
    pub action: Option<(Action, [u8; 32], u64)>,
    pub records: usize,
    pub durable: usize,
    pub record_failures: u64,
    pub record_stalled: bool,
    pub pending_age: u64,
    pub record_capacity: usize,
    pub ip_bytes: u64,
    pub admission_available: bool,
}

#[derive(Clone, Copy)]
#[allow(
    clippy::large_enum_variant,
    reason = "Inline snapshots avoid owner-side heap allocation; the request/reply queues have fixed capacity."
)]
pub(crate) enum Reply {
    View(Snapshot),
    Redirect,
    Authenticate {
        attempt: irongate_core::session::Attempt,
    },
    Script,
    Error(u16, &'static str),
}

pub(crate) struct Bridge {
    pub receiver: Receiver<Envelope>,
    pub record_sender: SyncSender<Record>,
    pub record_acks: Receiver<(usize, bool)>,
    pub wake: UnixDatagram,
}

pub(crate) struct WorkerBridge {
    pub sender: SyncSender<Envelope>,
    pub records: Receiver<Record>,
    pub record_acks: SyncSender<(usize, bool)>,
    pub wake: UnixDatagram,
}

impl Bridge {
    pub fn pair() -> std::io::Result<(Self, WorkerBridge)> {
        let (sender, receiver) = mpsc::sync_channel(8);
        let (record_sender, records) = mpsc::sync_channel(1);
        let (record_acks, acks) = mpsc::sync_channel(1);
        let (wake, notify) = UnixDatagram::pair()?;
        wake.set_nonblocking(true)?;
        notify.set_nonblocking(true)?;
        Ok((
            Self {
                receiver,
                record_sender,
                record_acks: acks,
                wake,
            },
            WorkerBridge {
                sender,
                records,
                record_acks,
                wake: notify,
            },
        ))
    }

    pub fn budget(&self, state: &mut crate::lab::state::State<'_>) {
        let mut ignored = [0; 64];
        for _ in 0..16 {
            if self.wake.recv(&mut ignored).is_err() {
                break;
            }
        }
        for _ in 0..2 {
            match self.record_acks.try_recv() {
                Ok((id, success)) => state.journal.complete(id, success, state.sessions.now()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    state.journal.unavailable = true;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        state.journal.tick(state.sessions.now());
        for _ in 0..8 {
            let Ok(envelope) = self.receiver.try_recv() else {
                break;
            };
            let reply = state.portal_request(envelope.peer, envelope.request);
            let _ = envelope.reply.try_send(reply);
        }
        if let Some(record) = state.journal.next(state.sessions.now())
            && self.record_sender.try_send(record).is_err()
        {
            state
                .journal
                .complete(record.id, false, state.sessions.now());
        }
    }
}
