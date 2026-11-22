//! Host inventory data, kept separate from configured-interface validation.

use crate::doctor::report::ReadIssue;

#[derive(Debug)]
pub(super) enum Detail<T> {
    Reported(T),
    Unknown(&'static str),
    Unavailable(ReadIssue),
    Unsupported(&'static str),
}

impl<T> Detail<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Reported(value) => Some(value),
            _ => None,
        }
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Reported(_) => "reported",
            Self::Unknown(_) => "unknown",
            Self::Unavailable(_) => "unavailable",
            Self::Unsupported(_) => "unsupported",
        }
    }
}

#[derive(Debug)]
pub(super) struct Card {
    pub name: String,
    pub kind: &'static str,
    pub admin_up: Detail<bool>,
    pub operstate: Detail<String>,
    pub carrier: Detail<bool>,
    pub link_active: Detail<bool>,
    pub mtu: Detail<u32>,
    pub driver: Detail<String>,
    pub speed_mbps: Detail<u32>,
    pub duplex: Detail<String>,
}

#[derive(Debug)]
pub(crate) struct Inventory {
    pub(super) source: &'static str,
    pub(super) observed_at_unix_seconds: Option<u64>,
    pub(super) discovered_interfaces: u64,
    pub(super) truncated: bool,
    pub(super) error_kind: Option<ReadIssue>,
    pub(super) unsupported_platform: bool,
    pub(super) cards: Vec<Card>,
}

impl Inventory {
    pub(super) fn new(source: &'static str) -> Self {
        Self {
            source,
            observed_at_unix_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|time| time.as_secs()),
            discovered_interfaces: 0,
            truncated: false,
            error_kind: None,
            unsupported_platform: false,
            cards: Vec::new(),
        }
    }

    pub(crate) fn complete(&self) -> bool {
        !self.truncated && self.error_kind.is_none() && !self.unsupported_platform
    }

    pub(crate) fn exit_code(&self) -> u8 {
        if self.unsupported_platform {
            return 3;
        }
        match self.error_kind {
            Some(ReadIssue::Permission) => 3,
            Some(_) => 4,
            None if self.truncated => 4,
            None => 0,
        }
    }

    pub(super) fn record_error(&mut self, issue: ReadIssue) {
        if self.error_kind.is_none() || issue == ReadIssue::Permission {
            self.error_kind = Some(issue);
        }
    }
}
