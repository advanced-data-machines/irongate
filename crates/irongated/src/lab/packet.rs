//! Validate peer/route identity before submitting any userspace transit.

use super::{profile::Side, state::State};
use irongate_proto::{arp, parse_request, routed};

impl State<'_> {
    pub fn packet(&mut self, side: Side, bytes: &[u8], output: &mut [u8]) -> Option<(Side, usize)> {
        self.counters.rx = self.counters.rx.saturating_add(1);
        let endpoint = side.arp();
        if bytes.len() < 14 || bytes[6..12] != endpoint.peer_mac {
            return None;
        }
        if bytes[12..14] == [8, 6] {
            let length = arp::reply(bytes, endpoint, output).ok()??;
            self.counters.arp = self.counters.arp.saturating_add(1);
            return Some((side, length));
        }
        if side == Side::Down
            && bytes.len() >= 42
            && bytes[23] == 17
            && bytes[34..38] == [0, 68, 0, 67]
        {
            let request = parse_request(bytes).ok()?;
            return self.dhcp(&request, output).map(|len| (side, len));
        }
        let length = match routed::rewrite(bytes, side.route(), output) {
            Ok(length) => length,
            Err(_) => {
                self.counters.invalid = self.counters.invalid.saturating_add(1);
                return None;
            }
        };
        if !self.allowed() {
            self.counters.denied = self.counters.denied.saturating_add(1);
            return None;
        }
        Some((side.opposite(), length))
    }
}
