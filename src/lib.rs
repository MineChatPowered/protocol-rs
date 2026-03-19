//! MineChat Protocol Library
//!
//! This library provides the MineChat protocol implementation for secure,
//! binary-framed, compressed client-server communication.
//!
//! ## Key Modules
//!
//! - `client`: Client-side protocol operations (linking, authentication)
//! - `packets`: Packet type definitions with serialization
//! - `protocol`: Error types and MessageStream trait
//! - `stream`: Stream implementations for different I/O backends
//! - `tls`: TLS implementations for secure communication
//! - `types`: Type definitions for messages and validation

#![warn(missing_docs)]
#![forbid(unsafe_code)]
/// Client-side protocol operations (linking, authentication)
pub mod client;
/// Packet type definitions with serialization
pub mod packets;
/// Error types and MessageStream trait
pub mod protocol;
/// Stream implementations for different I/O backends
pub mod stream;
/// TLS implementations for secure communication
pub mod tls;
/// Type definitions for messages and validation
pub mod types;

#[cfg(feature = "tokio")]
pub use client::{
    link_with_server, send_capabilities, send_chat_message, send_pong, wait_auth_ok,
};
#[cfg(feature = "tokio")]
pub use stream::TokioMessageStream;
#[cfg(feature = "tls-rustls")]
pub use tls::RustlsTlsMessageStream;
#[cfg(feature = "tls-native")]
pub use tls::TlsMessageStream;

pub use protocol::{
    MessageStream, MineChatError, chat_format, moderation_action, moderation_scope, packet_types,
};

pub use types::MessageContent;

pub use packets::{
    ClientUuid, LinkCode, MineChatPacket as Packet, MinecraftUuid, ModerationAction,
    ModerationScope, Payload, ValidationError,
};

pub use kyori_component_json;
