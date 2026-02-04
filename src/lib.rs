//! # minechat-protocol
//!
//! > A Rust library for MineChat protocol communication.
//!
//! This crate provides an asynchronous, runtime-independent API for interacting with a MineChat
//! server. It handles packet serialization (CBOR), compression (zstd), and framing,
//! allowing you to focus on the application logic.
//!
//! The core of the library is the [`MessageStream`] trait, which defines the interface for
//! sending and receiving [`MineChatPacket`]s. A default implementation for Tokio streams,
//! [`TokioMessageStream`], is provided under the `tokio` feature flag.
//!
//! ## Features
//!
//! - **Asynchronous**: Built for non-blocking I/O.
//! - **Runtime-Independent**: The core [`MessageStream`] trait can be implemented for any
//!   asynchronous runtime.
//! - **Efficient**: Packets are serialized with CBOR and compressed with zstd.
//! - **Protocol Compliant**: Implements MineChat Protocol v1.0.0 specification.
//! - **Authentication**: Provides mechanisms for client linking with the server.
//!
//! ## Examples
//!
//! ```rust,no_run
//! # #[cfg(feature = "tokio")]
//! # {
//! use minechat_protocol::{TokioMessageStream, link_with_server, send_chat_message};
//! use tokio::net::TcpStream;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let stream = TcpStream::connect("localhost:8080").await?;
//!     let mut message_stream = TokioMessageStream::new(stream);
//!
//!     // Link with server using code "ABC123"
//!     let (client_uuid, minecraft_uuid) = link_with_server(&mut message_stream, "ABC123").await?;
//!
//!     // Send a chat message
//!     send_chat_message(&mut message_stream, "commonmark", "Hello, world!").await?;
//!
//!     Ok(())
//! }
//! # }
//! ```
//!
//! ## Protocol Details
//!
//! Packets are framed with 4 bytes for decompressed size, 4 bytes for compressed size,
//! followed by the zstd-compressed, CBOR-serialized [`MineChatPacket`] payload.
//!
//! For more detailed information on packet structures and error types, refer to the
//! [`protocol`] module documentation.
//!
//! [`MessageStream`]: crate::protocol::MessageStream
//! [`MineChatPacket`]: crate::protocol::MineChatPacket
//! [`TokioMessageStream`]: crate::packets::TokioMessageStream
//! [`protocol`]: crate::protocol

#![warn(missing_docs)]
#![forbid(unsafe_code)]
/// Contains client-side protocol operations.
pub mod client;
/// Contains the implementation of the `TokioMessageStream` and packet handling functions.
pub mod packets;
/// Contains the core protocol definitions, including packet types, payloads, and the `MessageStream` trait.
pub mod protocol;
/// Contains stream implementations for different I/O backends.
pub mod stream;
/// Contains TLS implementations for secure communication.
pub mod tls;

// Re-export commonly used items
#[cfg(feature = "tls-rustls")]
pub use packets::RustlsTlsMessageStream;
#[cfg(feature = "tls-native")]
pub use packets::TlsMessageStream;
#[cfg(feature = "tokio")]
pub use packets::{
    TokioMessageStream, link_with_server, send_capabilities, send_chat_message, send_disconnect,
    send_pong,
};
pub use protocol::{
    AuthOkPayload, CapabilitiesPayload, ChatMessagePayload, DisconnectPayload, LinkOkPayload,
    LinkPayload, MineChatError, ModerationPayload, PingPayload, PongPayload, chat_format,
    moderation_action, moderation_scope, packet_types,
};
pub use protocol::{MessageStream, MineChatPacket, Payload};
