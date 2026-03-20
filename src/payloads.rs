//! Typed payload structs for each packet type.
//!
//! Each payload struct corresponds to a specific packet type as defined in the
//! MineChat protocol specification (Section 8). Field keys use integer indices
//! per the spec's CBOR encoding requirements.

use serde::{Deserialize, Serialize};

/// LINK payload (0x01) - Client → Server
///
/// Payload: `{ 0: linking_code, 1: client_uuid }`
///
/// Sent by the client during initial linking with the linking code
/// received from the Minecraft server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkPayload {
    /// The linking code from the Minecraft server
    pub linking_code: String,
    /// The client UUID identifying this device
    pub client_uuid: String,
}

/// LINK_OK payload (0x02) - Server → Client
///
/// Payload: `{ 0: minecraft_uuid }`
///
/// Sent by the server when linking succeeds, containing the
/// Minecraft account UUID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkOkPayload {
    /// The Minecraft UUID of the linked account
    pub minecraft_uuid: String,
}

/// CAPABILITIES payload (0x03) - Client → Server
///
/// Payload: `{ 0: supports_components }`
///
/// Sent by the client immediately after linking or reconnecting
/// to declare supported features.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesPayload {
    /// Whether the client supports rich text components
    pub supports_components: bool,
}

/// AUTH_OK payload (0x04) - Server → Client
///
/// Payload: `{}` (empty)
///
/// Indicates that authentication is complete and the client
/// may begin sending messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthOkPayload {}

/// CHAT_MESSAGE payload (0x05) - Bidirectional
///
/// Payload: `{ 0: format, 1: content }`
///
/// Carries chat messages between client and server. The format
/// field indicates whether content is CommonMark or Minecraft
/// text components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    /// The message format ("commonmark" or "components")
    pub format: String,
    /// The message content
    pub content: String,
}

/// PING payload (0x06) - Bidirectional
///
/// Payload: `{ 0: timestamp_ms }`
///
/// Keep-alive packet for connection maintenance and RTT measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingPayload {
    /// Timestamp in milliseconds
    pub timestamp_ms: i64,
}

/// PONG payload (0x07) - Bidirectional
///
/// Payload: `{ 0: timestamp_ms }`
///
/// Response to PING packet. The timestamp MUST match the
/// corresponding PING.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongPayload {
    /// Timestamp in milliseconds (must match the corresponding PING)
    pub timestamp_ms: i64,
}

/// MODERATION payload (0x08) - Server → Client
///
/// Payload: `{ 0: action, 1: scope, 2: reason?, 3: duration_seconds? }`
///
/// Used by the server to enforce moderation actions such as warnings,
/// mutes, kicks, or bans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationPayload {
    /// The moderation action (0=warn, 1=mute, 2=kick, 3=ban)
    pub action: i32,
    /// The scope of the moderation (0=client, 1=account)
    pub scope: i32,
    /// Optional reason for the moderation action
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional duration in seconds (for mute/ban)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i32>,
}

/// SYSTEM_DISCONNECT payload (0x09) - Server → Client
///
/// Payload: `{ 0: reason_code, 1: message }`
///
/// Indicates that the server is intentionally terminating the connection
/// due to a system-level lifecycle event (shutdown, maintenance, etc.).
/// After sending this packet, the server MUST immediately close the
/// underlying TCP connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDisconnectPayload {
    /// The reason code (0=shutdown, 1=maintenance, 2=internal_error, 3=overloaded)
    pub reason_code: i32,
    /// Human-readable message describing the disconnect
    pub message: String,
}
