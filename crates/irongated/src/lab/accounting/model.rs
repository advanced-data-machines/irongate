//! Fixed-size lifecycle data. Usernames are accounting identity, never passwords.
use irongate_core::{lease, session};
#[derive(Clone, Copy)]
pub(crate) struct UserName {
    bytes: [u8; 64],
    len: u8,
}
impl UserName {
    pub fn new(bytes: &[u8]) -> Option<Self> {
        if bytes.is_empty() || bytes.len() > 64 || !bytes.iter().all(|b| (32..=126).contains(b)) {
            return None;
        }
        let mut value = Self {
            bytes: [0; 64],
            len: bytes.len() as u8,
        };
        value.bytes[..bytes.len()].copy_from_slice(bytes);
        Some(value)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Octets {
    pub input: u64,
    pub output: u64,
    pub overflow: bool,
}
impl Octets {
    pub fn add(&mut self, input: bool, bytes: u64) {
        let target = if input {
            &mut self.input
        } else {
            &mut self.output
        };
        match target.checked_add(bytes) {
            Some(value) => *target = value,
            None => {
                *target = u64::MAX;
                self.overflow = true;
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Start,
    Interim,
    Stop,
}
#[derive(Clone, Copy)]
pub(crate) struct Record {
    pub user: UserName,
    pub session_id: [u8; 58],
    pub kind: Kind,
    pub created: u64,
    pub session_seconds: u64,
    pub octets: Octets,
    pub cause: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub slot: usize,
    pub serial: u64,
    pub sequence: u64,
}
#[derive(Clone, Copy)]
pub(crate) struct Dispatch {
    pub token: Token,
    pub record: Record,
    pub delay: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Ack,
    Timeout,
    Invalid,
    Unavailable,
}
#[derive(Clone, Copy)]
pub(crate) struct Receipt {
    pub token: Token,
    pub outcome: Outcome,
    pub invalid: u16,
}
#[derive(Clone, Copy)]
pub(super) struct Flight {
    pub dispatch: Dispatch,
    pub sent: u64,
}
#[derive(Clone, Copy)]
pub(super) struct Life {
    pub serial: u64,
    pub handle: session::Handle,
    pub lease: lease::Handle,
    pub activated: bool,
    pub started: u64,
    pub next_interim: u64,
    pub totals: Octets,
    pub start: Record,
    pub start_acked: bool,
    pub interim: Option<Record>,
    pub stop: Option<Record>,
    pub retry_at: u64,
    pub failures: u8,
}
impl Life {
    pub fn snapshot(&self, kind: Kind, now: u64, cause: u32) -> Record {
        Record {
            kind,
            created: now,
            session_seconds: now.saturating_sub(self.started),
            octets: self.totals,
            cause,
            ..self.start
        }
    }
    pub fn pending(&self) -> Option<Record> {
        if !self.activated {
            None
        } else if !self.start_acked {
            Some(self.start)
        } else {
            self.stop.or(self.interim)
        }
    }
}
