//! FluxDNS
//!
//! A modern DNS proxy service supporting multiple protocols (UDP, DoT, DoH, DoQ, DoH3)
//! with a web management interface.

mod bootstrap;
mod config;
mod db;
mod dns;
mod error;
mod llm;
mod log;
mod state;
mod web;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap::run().await
}
