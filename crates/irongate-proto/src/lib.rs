//! IronGate's planned protocol parsing and encoding layer.
//!
//! This crate is intentionally an empty scaffold. Parsers will borrow input
//! slices; encoders will write into caller-provided bounded buffers. Evaluate
//! existing allocation-free protocol libraries before implementing protocols.

#![no_std]
#![forbid(unsafe_code)]
