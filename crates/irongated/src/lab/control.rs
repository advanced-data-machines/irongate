//! Root-owned guest-only test controls; not a portal or public authorization API.

use super::state::State;
use serde_json::{Value, json};

impl State<'_> {
    pub fn control(&mut self, command: &[u8]) -> Value {
        let result = match command {
            b"status" => Ok(()),
            b"consent" => self.consent(),
            b"record-pressure" => self
                .active
                .ok_or_else(|| "no active session".into())
                .and_then(|handle| {
                    let view = self
                        .sessions
                        .view(handle)
                        .ok_or_else(|| "no live session".to_owned())?;
                    self.journal
                        .pressure(handle, view.lease, self.sessions.now());
                    Ok(())
                }),
            b"logout" => self
                .active
                .ok_or_else(|| "no active session".into())
                .and_then(|handle| {
                    self.sessions
                        .logout(handle)
                        .map_err(|e| format!("logout: {e:?}"))
                })
                .and_then(|()| self.pump()),
            _ => Err("unknown lab command".into()),
        };
        let view = self.active.and_then(|handle| self.sessions.view(handle));
        let c = &self.counters;
        json!({"schema":1,"scope":"isolated_packet_lab","ok":result.is_ok(),"error":result.err(),
            "test_authorization":true,"allowed":self.allowed(),"now":self.sessions.now(),
            "phase":view.map(|v|format!("{:?}",v.phase)),"until":view.and_then(|v|v.authorized_until),
            "local_records":{"retained":self.journal.used,"durable":self.journal.durable,"pending":self.journal.used-self.journal.durable,"write_failures":self.journal.failed},
            "counters":{"rx":c.rx,"invalid":c.invalid,"denied":c.denied,"tx_errors":c.tx_errors,
            "control_errors":c.control_errors,"forwarded":c.forwarded,"forwarded_ip_bytes":c.forwarded_ip_bytes,"arp":c.arp,
            "offers":c.offers,"acks":c.acks,"naks":c.naks,"applies":c.applies,"removals":c.removals}})
    }
    fn consent(&mut self) -> Result<(), String> {
        let handle = self.active.ok_or_else(|| "no active session".to_owned())?;
        let attempt = self
            .sessions
            .begin_consent(&self.leases, handle)
            .map_err(|e| format!("consent start: {e:?}"))?;
        self.sessions
            .consent(&self.leases, attempt, 5)
            .map_err(|e| format!("consent decision: {e:?}"))?;
        self.pump()
    }
}
