//! Fixed one-use action cells. Entropy comes from the trusted HTTP worker.

use irongate_core::{lease, session};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Consent,
    Logout,
}

#[derive(Clone, Copy)]
struct Entry {
    value: [u8; 32],
    session: session::Handle,
    lease: lease::Handle,
    action: Action,
    until: u64,
}

#[derive(Default)]
pub(crate) struct Tokens {
    entries: [Option<Entry>; 4],
}

impl Tokens {
    pub(super) fn issue(
        &mut self,
        value: [u8; 32],
        session: session::Handle,
        lease: lease::Handle,
        action: Action,
        now: u64,
    ) -> Option<([u8; 32], u64)> {
        self.expire(now);
        if let Some(entry) = self
            .entries
            .iter()
            .flatten()
            .find(|e| e.session == session && e.lease == lease && e.action == action)
        {
            return Some((entry.value, entry.until));
        }
        if value == [0; 32] || self.entries.iter().flatten().any(|e| e.value == value) {
            return None;
        }
        let until = now.checked_add(60)?;
        let cell = self.entries.iter_mut().find(|cell| cell.is_none())?;
        *cell = Some(Entry {
            value,
            session,
            lease,
            action,
            until,
        });
        Some((value, until))
    }

    pub(super) fn consume(
        &mut self,
        value: [u8; 32],
        session: session::Handle,
        lease: lease::Handle,
        action: Action,
        now: u64,
    ) -> bool {
        self.expire(now);
        // Fixed 32-byte comparison; never format/log bearer values.
        let cell = self
            .entries
            .iter_mut()
            .find(|cell| cell.is_some_and(|e| equal(e.value, value)));
        let Some(cell) = cell else {
            return false;
        };
        let entry = cell.take().expect("matched live token");
        entry.session == session
            && entry.lease == lease
            && entry.action == action
            && now < entry.until
    }

    pub(crate) fn invalidate(&mut self) {
        self.entries.fill(None);
    }
    pub(crate) fn expire(&mut self, now: u64) {
        for cell in &mut self.entries {
            if cell.is_some_and(|e| now >= e.until) {
                *cell = None;
            }
        }
    }
}

fn equal(a: [u8; 32], b: [u8; 32]) -> bool {
    a.iter()
        .zip(b)
        .fold(0u8, |difference, (a, b)| difference | (*a ^ b))
        == 0
}
