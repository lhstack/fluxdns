//! FluxDNS
//!
//! A modern DNS proxy service supporting multiple protocols (UDP, DoT, DoH, DoQ, DoH3)
//! with a web management interface.

mod application;
mod business;
mod dns;
mod infrastructure;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    application::bootstrap::run().await
}
