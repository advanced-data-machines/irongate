//! One untagged, deny-by-default guest lab. Normal daemon deployment is pending.

mod control;
mod dhcp;
mod io;
mod owner;
mod packet;
mod profile;
mod state;
mod transport;

pub use io::run;

#[cfg(test)]
mod tests;
