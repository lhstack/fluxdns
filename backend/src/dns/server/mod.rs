//! DNS Server Module
//!
//! Provides DNS server implementations for multiple protocols:
//! - UDP: Standard DNS over UDP (port 53)
//! - DoT: DNS over TLS (port 853)
//! - DoH: DNS over HTTPS (port 443)
//! - DoQ: DNS over QUIC (port 8853)

mod udp;
mod dot;
mod doh;
mod doq;

#[cfg(test)]
mod protocol_consistency_tests;

pub use udp::*;
#[allow(unused_imports)]
pub use dot::*;
pub use doh::*;
#[allow(unused_imports)]
pub use doq::*;
