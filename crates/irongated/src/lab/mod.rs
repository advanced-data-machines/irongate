//! One untagged, deny-by-default guest lab. Normal daemon deployment is pending.

mod control;
mod dhcp;
mod io;
mod packet;
mod profile;
mod state;

pub use io::run;

#[cfg(test)]
mod tests;
