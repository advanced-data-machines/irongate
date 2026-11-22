//! Offline admission owner with caller-owned slots and retained policy effects.
//!
//! Advance the lease clock first, then this owner. Dispatch each effect once,
//! acknowledge only after its operation finishes, and retry failed removals.
//! No platform enforcement, provider credentials or durable journal lives here.

mod admission;
mod effects;
mod identity;
mod lifecycle;
mod model;
mod storage;
mod table;

pub use identity::{Attempt, EffectToken, Handle};
pub use model::{Config, Effect, EffectKind, EndReason, Error, MAX_SESSIONS, Phase, View};
pub use storage::Slot;
pub use table::Table;

#[cfg(test)]
mod tests;
