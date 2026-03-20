//! TLS module for MineChat protocol.
//!
//! This module provides TLS support via different backends:
//! - `tls-native`: Uses native-tls (OpenSSL on most platforms)
//! - `tls-rustls`: Uses rustls (pure Rust, no native dependencies)

#[cfg(feature = "tls-native")]
pub mod native;

#[cfg(feature = "tls-rustls")]
pub mod rustls;

pub use crate::protocol::{MessageStream, MineChatError};
