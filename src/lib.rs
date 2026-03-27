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
//! - `tls`: TLS implementations for secure communication (rustls)
//! - `types`: Type definitions for messages and validation
//! - `payloads`: Typed payload structs for each packet type
//! - `cbor`: Custom CBOR serialization with integer keys
//! - `ansi`: ANSI escape code rendering for terminal colors

#![warn(missing_docs)]
#![forbid(unsafe_code)]
/// Custom CBOR serialization with spec-compliant integer keys
pub mod ansi;
/// Custom CBOR serialization with spec-compliant integer keys
pub mod cbor;
/// Client-side protocol operations (linking, authentication)
pub mod client;
/// Packet type definitions with serialization
pub mod packets;
/// Typed payload structs for each packet type
pub mod payloads;
/// Error types and MessageStream trait
pub mod protocol;
/// Stream implementations for different I/O backends
pub mod stream;
/// TLS implementations for secure communication
pub mod tls;
/// Type definitions for messages and validation
pub mod types;

pub use ansi::message_content_to_ansi;
pub use client::{link_with_server, send_capabilities, send_chat_message, send_pong, wait_auth_ok};
pub use stream::TokioMessageStream;
pub use tls::rustls::RustlsTlsMessageStream;

pub use protocol::{
    MessageStream, MineChatError, chat_format, moderation_action, moderation_scope, packet_types,
};

pub use types::MessageContent;

pub use packets::{
    ClientUuid, LinkCode, MessageFormat, MineChatPacket as Packet, MinecraftUuid, ModerationAction,
    ModerationScope, ValidationError, packet_type, system_disconnect_reason,
};

pub use kyori_component_json as components;
