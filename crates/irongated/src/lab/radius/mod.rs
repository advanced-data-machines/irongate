//! One bounded PAP/UDP interoperability profile, restricted to the isolated lab.
pub(crate) mod client;
pub(crate) mod config;
pub(crate) mod credentials;
mod crypto;
pub(crate) mod health;
mod request;
mod response;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Accept { seconds: u32 },
    Reject,
    Unsupported,
    Timeout,
    Invalid,
    Unavailable,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Decision {
    pub outcome: Outcome,
    pub elapsed_ms: u32,
    pub invalid: u16,
    pub sends: u8,
}

#[cfg(test)]
mod tests;
