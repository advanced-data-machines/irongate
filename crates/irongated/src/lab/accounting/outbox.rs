//! Each retained lifecycle owns Start, replaceable Interim and reserved Stop.
use super::{model::*, status::Summary};
use irongate_core::{lease, session};
pub(super) const CAPACITY: usize = 4;
pub(crate) struct Outbox {
    pub(super) boot: Option<[u8; 16]>,
    pub(super) interval: u64,
    pub(super) lives: [Option<Life>; CAPACITY],
    pub(super) active: Option<usize>,
    pub(super) serial: u64,
    pub(super) sequence: u64,
    pub(super) flight: Option<Flight>,
    pub(super) summary: Summary,
}
impl Default for Outbox {
    fn default() -> Self {
        Self {
            boot: None,
            interval: 2,
            lives: [None; CAPACITY],
            active: None,
            serial: 0,
            sequence: 0,
            flight: None,
            summary: Summary::default(),
        }
    }
}
impl Outbox {
    pub fn enabled(boot: [u8; 16], interval: u64) -> Self {
        assert!((1..=60).contains(&interval));
        Self {
            boot: Some(boot),
            interval,
            ..Self::default()
        }
    }
    pub fn is_enabled(&self) -> bool {
        self.boot.is_some()
    }
    pub fn can_admit(&self) -> bool {
        !self.is_enabled()
            || (!self.summary.unavailable
                && !self.summary.stalled
                && self.serial < u64::MAX
                && self.sequence < u64::MAX
                && self.active.is_none()
                && self.lives.iter().any(Option::is_none))
    }
    pub fn reserve(
        &mut self,
        handle: session::Handle,
        lease: lease::Handle,
        user: UserName,
        now: u64,
    ) -> bool {
        if !self.is_enabled() {
            return true;
        }
        if !self.can_admit() {
            return false;
        }
        let index = self
            .lives
            .iter()
            .position(Option::is_none)
            .expect("admission checked");
        self.serial += 1;
        let mut id = [b'0'; 58];
        hex(&self.boot.expect("enabled"), &mut id[..32]);
        id[32] = b'-';
        hex(&1u32.to_be_bytes(), &mut id[33..41]);
        id[41] = b'-';
        hex(&self.serial.to_be_bytes(), &mut id[42..]);
        self.lives[index] = Some(Life {
            serial: self.serial,
            handle,
            lease,
            activated: false,
            started: now,
            next_interim: now.saturating_add(self.interval),
            totals: Octets::default(),
            start: Record {
                user,
                session_id: id,
                kind: Kind::Start,
                created: now,
                session_seconds: 0,
                octets: Octets::default(),
                cause: 0,
            },
            start_acked: false,
            interim: None,
            stop: None,
            retry_at: now,
            failures: 0,
        });
        self.active = Some(index);
        true
    }
    pub fn cancel(&mut self) {
        if let Some(index) = self.active
            && self.lives[index].is_some_and(|l| !l.activated)
        {
            self.lives[index] = None;
            self.active = None;
        }
    }
    pub fn activate(&mut self, handle: session::Handle, lease: lease::Handle, now: u64) {
        if let Some(life) = self.current(handle, lease)
            && !life.activated
        {
            life.activated = true;
            life.started = now;
            life.start.created = now;
        }
    }
    pub fn submitted(
        &mut self,
        handle: session::Handle,
        lease: lease::Handle,
        input: bool,
        bytes: u64,
    ) {
        if let Some(life) = self.current(handle, lease)
            && life.activated
        {
            life.totals.add(input, bytes);
        }
    }
    pub fn stop(&mut self, handle: session::Handle, lease: lease::Handle, now: u64, cause: u32) {
        if let Some(life) = self.current(handle, lease) {
            if life.activated {
                life.stop = Some(life.snapshot(Kind::Stop, now, cause));
                life.interim = None;
            } else {
                self.cancel();
                return;
            }
            self.active = None;
        }
    }
    fn current(&mut self, handle: session::Handle, lease: lease::Handle) -> Option<&mut Life> {
        self.active
            .and_then(|i| self.lives[i].as_mut())
            .filter(|l| l.handle == handle && l.lease == lease)
    }
}
fn hex(input: &[u8], output: &mut [u8]) {
    let digits = b"0123456789abcdef";
    for (byte, pair) in input.iter().zip(output.as_chunks_mut::<2>().0) {
        pair[0] = digits[(byte >> 4) as usize];
        pair[1] = digits[(byte & 15) as usize];
    }
}
