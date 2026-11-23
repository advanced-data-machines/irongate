//! Retain a bounded audit trail, reserving termination capacity before admission.

use irongate_core::{lease, session};

pub(super) const CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Granted,
    Revoked,
    Expired,
    LeaseLost,
    TestPressure,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Record {
    pub id: usize,
    pub at: u64,
    pub kind: Kind,
    pub session: session::Handle,
    pub lease: lease::Handle,
    pub input_output_ip_bytes: u64,
}

pub(crate) struct Journal {
    entries: [Option<Record>; CAPACITY],
    pub used: usize,
    pub durable: usize,
    pub failed: u64,
    pub in_flight: Option<usize>,
    pub retry_at: u64,
    pub reserved: Option<(session::Handle, lease::Handle)>,
    pub unavailable: bool,
    pub stalled: bool,
    pub in_flight_since: u64,
}

impl Default for Journal {
    fn default() -> Self {
        Self {
            entries: [None; CAPACITY],
            used: 0,
            durable: 0,
            failed: 0,
            in_flight: None,
            retry_at: 0,
            reserved: None,
            unavailable: false,
            stalled: false,
            in_flight_since: 0,
        }
    }
}

impl Journal {
    pub fn can_admit(&self) -> bool {
        !self.unavailable && self.reserved.is_none() && self.used + 2 <= CAPACITY
    }

    pub fn grant(
        &mut self,
        session: session::Handle,
        lease: lease::Handle,
        at: u64,
        bytes: u64,
    ) -> bool {
        if !self.can_admit() {
            return false;
        }
        self.reserved = Some((session, lease));
        self.push(session, lease, at, bytes, Kind::Granted);
        true
    }

    pub fn stop(&mut self, at: u64, bytes: u64, kind: Kind) {
        if let Some((session, lease)) = self.reserved.take() {
            self.push(session, lease, at, bytes, kind);
        }
    }

    fn push(
        &mut self,
        session: session::Handle,
        lease: lease::Handle,
        at: u64,
        bytes: u64,
        kind: Kind,
    ) {
        assert!(self.used < CAPACITY, "admission reserves its stop cell");
        self.entries[self.used] = Some(Record {
            id: self.used + 1,
            at,
            kind,
            session,
            lease,
            input_output_ip_bytes: bytes,
        });
        self.used += 1;
    }

    pub fn next(&mut self, now: u64) -> Option<Record> {
        if self.in_flight.is_some() || self.durable == self.used || now < self.retry_at {
            return None;
        }
        let record = self.entries[self.durable]?;
        self.in_flight = Some(record.id);
        self.in_flight_since = now;
        Some(record)
    }

    pub fn complete(&mut self, id: usize, success: bool, now: u64) {
        if self.in_flight != Some(id) {
            return;
        }
        self.in_flight = None;
        self.stalled = false;
        if success {
            self.durable = id;
            self.unavailable = false;
        } else {
            self.failed = self.failed.saturating_add(1);
            self.retry_at = now.saturating_add(1);
            self.unavailable = true;
        }
    }

    pub fn tick(&mut self, now: u64) {
        if self.in_flight.is_some() && now.saturating_sub(self.in_flight_since) >= 2 {
            self.stalled = true;
            self.unavailable = true;
        }
    }

    pub fn pending_age(&self, now: u64) -> u64 {
        if self.durable < self.used {
            self.entries[self.durable].map_or(0, |record| now.saturating_sub(record.at))
        } else {
            0
        }
    }

    // Explicit root-only lab fault injection, never a client API.
    pub fn pressure(&mut self, session: session::Handle, lease: lease::Handle, at: u64) {
        while self.used + usize::from(self.reserved.is_some()) < CAPACITY {
            self.push(session, lease, at, 0, Kind::TestPressure);
        }
    }
}
