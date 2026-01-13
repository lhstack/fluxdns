//! DNS Proxy Module
//!
//! Provides DNS proxy functionality including:
//! - Upstream server management
//! - Multiple protocol support (UDP, DoT, DoH, DoQ)
//! - Query strategies (concurrent, fastest, round-robin, random)
//! - Failover handling

mod upstream;
mod client;
mod strategy;

#[cfg(test)]
mod forwarding_tests;

pub use upstream::*;
#[allow(unused_imports)]
pub use client::*;
pub use strategy::*;
