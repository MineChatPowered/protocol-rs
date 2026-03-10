//! Protocol error types and traits.

use async_trait::async_trait;
use miette::Diagnostic;
use std::io;
use thiserror::Error;

use crate::packets::MineChatPacket;

/// Errors that can occur during the operation of the MineChat protocol.
#[derive(Debug, Error, Diagnostic)]
pub enum MineChatError {
    /// I/O error. Contains the underlying error.
    #[error("I/O error: {0}")]
    #[diagnostic(code(minechat::io))]
    Io(#[from] io::Error),

    /// Serde error. Contains the underlying CBOR error.
    #[error("Serde error: {0}")]
    #[diagnostic(code(minechat::serde))]
    Serde(#[from] serde_cbor::Error),

    /// Serde JSON error. Contains the underlying JSON error.
    #[error("Serde JSON error: {0}")]
    #[diagnostic(code(minechat::serde_json))]
    SerdeJson(#[from] serde_json::Error),

    /// Zstd error.
    #[error("Zstd error: {0}")]
    #[diagnostic(code(minechat::zstd))]
    Zstd(String),

    /// Server not linked.
    #[error("Server not linked")]
    ServerNotLinked,

    /// Configuration error.
    #[error("Config error: {0}")]
    #[diagnostic(code(minechat::config_error), help = "Check your configuration file")]
    ConfigError(String),

    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    #[diagnostic(
        code(minechat::auth_failed),
        help = "Try logging in again with valid credentials"
    )]
    AuthFailed(String),

    /// UUID error.
    #[error("UUID error: {0}")]
    #[diagnostic(code(minechat::uuid))]
    Uuid(#[from] uuid::Error),

    /// Disconnected.
    #[error("Disconnected")]
    #[diagnostic(
        code(minechat::disconnected),
        help = "If this is unexpected, try reconnecting"
    )]
    Disconnected,

    /// Invalid packet type.
    #[error("Invalid packet type: {0}")]
    InvalidPacketType(i32),

    /// Invalid frame size.
    #[error("Invalid frame size: {0}")]
    InvalidFrameSize(i32),
}

/// A trait for sending and receiving `MineChatPacket`s over an asynchronous stream.
///
/// This trait abstracts over the underlying transport, allowing for different
/// implementations (e.g., Tokio TCP streams, in-memory streams for testing).
#[async_trait]
pub trait MessageStream {
    /// Sends a `MineChatPacket` over the stream.
    ///
    /// The packet is serialized using CBOR, compressed with zstd, and then framed
    /// with decompressed and compressed lengths before being written to the stream.
    ///
    /// # Arguments
    ///
    /// * `packet` - A reference to the `MineChatPacket` to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the packet was sent successfully, or a `MineChatError` otherwise.
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError>;

    /// Receives a `MineChatPacket` from the stream.
    ///
    /// This method reads the message framing (decompressed and compressed lengths),
    /// reads the compressed payload, decompresses it with zstd, and then deserializes
    /// it from CBOR into a `MineChatPacket`.
    ///
    /// # Returns
    ///
    /// `Ok(MineChatPacket)` if a packet was received and parsed successfully,
    /// or a `MineChatError` otherwise.
    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError>;
}

/// Packet type constants
pub mod packet_types {
    /// `LINK` packet (Client → Server)
    pub const LINK: i32 = 0x01;
    /// `LINK_OK` packet (Server → Client)
    pub const LINK_OK: i32 = 0x02;
    /// `CAPABILITIES` packet (Client → Server)
    pub const CAPABILITIES: i32 = 0x03;
    /// `AUTH_OK` packet (Server → Client)
    pub const AUTH_OK: i32 = 0x04;
    /// `CHAT_MESSAGE` packet (Bidirectional)
    pub const CHAT_MESSAGE: i32 = 0x05;
    /// `PING` packet (Bidirectional)
    pub const PING: i32 = 0x06;
    /// `PONG` packet (Bidirectional)
    pub const PONG: i32 = 0x07;
    /// `MODERATION` packet (Server → Client)
    pub const MODERATION: i32 = 0x08;
    /// `DISCONNECT` packet (Bidirectional) - Implementation-private
    pub const DISCONNECT: i32 = 0x80;
}

/// Chat format constants
pub mod chat_format {
    /// `commonmark` - CommonMark Markdown format
    pub const COMMONMARK: &str = "commonmark";
    /// `components` - JSON-encoded Minecraft Text Component format
    pub const COMPONENTS: &str = "components";
}

/// Moderation action constants
pub mod moderation_action {
    /// `WARN` action (0)
    pub const WARN: i32 = 0;
    /// `MUTE` action (1)
    pub const MUTE: i32 = 1;
    /// `KICK` action (2)
    pub const KICK: i32 = 2;
    /// `BAN` action (3)
    pub const BAN: i32 = 3;
}

/// Moderation scope constants
pub mod moderation_scope {
    /// `CLIENT` scope (0) - affects only the current device
    pub const CLIENT: i32 = 0;
    /// `ACCOUNT` scope (1) - affects the entire Minecraft account
    pub const ACCOUNT: i32 = 1;
}
