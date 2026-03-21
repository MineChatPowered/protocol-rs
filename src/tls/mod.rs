//! TLS module for MineChat protocol.
//!
//! This module provides TLS support via rustls (pure Rust, no native dependencies).

pub mod rustls;

pub use crate::protocol::{MessageStream, MineChatError};
