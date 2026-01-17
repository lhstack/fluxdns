// LLM Module - OpenAI Compatible Client for Multi-Provider Support
//
// This module provides a unified interface for interacting with various LLM providers
// that are compatible with the OpenAI API specification.

pub mod client;
pub mod config;
pub mod functions;
pub mod types;

pub use client::LlmClient;
pub use functions::FunctionRegistry;
