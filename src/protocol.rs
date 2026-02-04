use async_trait::async_trait;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

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
    /// LINK packet (Client → Server)
    pub const LINK: i32 = 0x01;
    /// LINK_OK packet (Server → Client)
    pub const LINK_OK: i32 = 0x02;
    /// CAPABILITIES packet (Client → Server)
    pub const CAPABILITIES: i32 = 0x03;
    /// AUTH_OK packet (Server → Client)
    pub const AUTH_OK: i32 = 0x04;
    /// CHAT_MESSAGE packet (Bidirectional)
    pub const CHAT_MESSAGE: i32 = 0x05;
    /// PING packet (Bidirectional)
    pub const PING: i32 = 0x06;
    /// PONG packet (Bidirectional)
    pub const PONG: i32 = 0x07;
    /// MODERATION packet (Server → Client)
    pub const MODERATION: i32 = 0x08;
    /// DISCONNECT packet (Bidirectional)
    pub const DISCONNECT: i32 = 0x09;
}

/// Chat format constants
pub mod chat_format {
    /// CommonMark Markdown format
    pub const COMMONMARK: &str = "commonmark";
    /// JSON-encoded Minecraft Text Component format
    pub const COMPONENTS: &str = "components";
}

/// Moderation action constants
pub mod moderation_action {
    /// Warn action
    pub const WARN: i32 = 0;
    /// Mute action
    pub const MUTE: i32 = 1;
    /// Kick action
    pub const KICK: i32 = 2;
    /// Ban action
    pub const BAN: i32 = 3;
}

/// Moderation scope constants
pub mod moderation_scope {
    /// Client UUID scope
    pub const CLIENT: i32 = 0;
    /// Minecraft UUID scope
    pub const ACCOUNT: i32 = 1;
}

/// A MineChat packet with type and payload.
///
/// All packets follow the common envelope structure: { packet_type: int, payload: map }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MineChatPacket {
    /// The packet type identifier.
    pub packet_type: i32,
    /// The packet payload.
    pub payload: Payload,
}

/// Packet payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
    /// LINK payload
    Link(LinkPayload),
    /// LINK_OK payload
    LinkOk(LinkOkPayload),
    /// CAPABILITIES payload
    Capabilities(CapabilitiesPayload),
    /// AUTH_OK payload
    AuthOk(AuthOkPayload),
    /// CHAT_MESSAGE payload
    ChatMessage(ChatMessagePayload),
    /// PING payload
    Ping(PingPayload),
    /// PONG payload
    Pong(PongPayload),
    /// MODERATION payload
    Moderation(ModerationPayload),
    /// DISCONNECT payload
    Disconnect(DisconnectPayload),
    /// Empty payload
    Empty,
}

/// LINK payload (0x01)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPayload {
    /// The linking code
    pub linking_code: String,
    /// The client UUID
    pub client_uuid: String,
}

/// LINK_OK payload (0x02)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkOkPayload {
    /// The Minecraft UUID
    pub minecraft_uuid: String,
}

/// CAPABILITIES payload (0x03)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesPayload {
    /// Whether the client supports components
    pub supports_components: bool,
}

/// AUTH_OK payload (0x04) - empty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthOkPayload {}

/// CHAT_MESSAGE payload (0x05)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    /// The message format ("commonmark" or "components")
    pub format: String,
    /// The message content
    pub content: String,
}

/// PING payload (0x06)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingPayload {
    /// The timestamp in milliseconds
    pub timestamp_ms: i64,
}

/// PONG payload (0x07)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongPayload {
    /// The timestamp in milliseconds (must match the corresponding PING)
    pub timestamp_ms: i64,
}

/// MODERATION payload (0x08)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationPayload {
    /// The moderation action (warn=0, mute=1, kick=2, ban=3)
    pub action: i32,
    /// The scope (client=0, account=1)
    pub scope: i32,
    /// Optional reason
    pub reason: Option<String>,
    /// Optional duration in seconds
    pub duration_seconds: Option<i32>,
}

/// DISCONNECT payload (0x09)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectPayload {
    /// The reason for disconnection
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_packet_serialization() {
        let packet = MineChatPacket {
            packet_type: packet_types::LINK,
            payload: Payload::Link(LinkPayload {
                linking_code: "test-code".to_string(),
                client_uuid: "test-uuid".to_string(),
            }),
        };
        let serialized = serde_cbor::to_vec(&packet).unwrap();
        let deserialized: MineChatPacket = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.packet_type, packet_types::LINK);
        if let Payload::Link(payload) = deserialized.payload {
            assert_eq!(payload.linking_code, "test-code");
            assert_eq!(payload.client_uuid, "test-uuid");
        } else {
            panic!("Deserialized payload is not Link");
        }
    }

    #[test]
    fn test_chat_message_packet_serialization() {
        let packet = MineChatPacket {
            packet_type: packet_types::CHAT_MESSAGE,
            payload: Payload::ChatMessage(ChatMessagePayload {
                format: chat_format::COMMONMARK.to_string(),
                content: "Hello, world!".to_string(),
            }),
        };
        let serialized = serde_cbor::to_vec(&packet).unwrap();
        let deserialized: MineChatPacket = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.packet_type, packet_types::CHAT_MESSAGE);
        if let Payload::ChatMessage(payload) = deserialized.payload {
            assert_eq!(payload.format, chat_format::COMMONMARK);
            assert_eq!(payload.content, "Hello, world!");
        } else {
            panic!("Deserialized payload is not ChatMessage");
        }
    }

    #[test]
    fn test_ping_pong_packet_serialization() {
        let ping_packet = MineChatPacket {
            packet_type: packet_types::PING,
            payload: Payload::Ping(PingPayload {
                timestamp_ms: 1234567890,
            }),
        };
        let serialized = serde_cbor::to_vec(&ping_packet).unwrap();
        let deserialized: MineChatPacket = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.packet_type, packet_types::PING);
        if let Payload::Ping(payload) = deserialized.payload {
            assert_eq!(payload.timestamp_ms, 1234567890);
        } else {
            panic!("Deserialized payload is not Ping");
        }
    }
}
