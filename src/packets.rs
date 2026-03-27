//! MineChat packet types and serialization.
//!
//! This module defines all packet types for the MineChat protocol as specified
//! in the protocol specification (Section 8). Each packet consists of an envelope
//! with a packet type identifier and a typed payload.
//!
//! ## Serialization
//!
//! All packets are serialized as CBOR maps with integer keys:
//!
//! ```cbor
//! { 0: packet_type, 1: payload }
//! ```
//!
//! The payload structure varies by packet type as defined in the specification.

use crate::payloads;

use crate::cbor;
use crate::types::MessageContent;
use miette::Diagnostic;
use payloads::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Packet type constants as defined in the MineChat protocol specification.
pub mod packet_type {
    /// `LINK` packet type (0x01) - Client → Server
    pub const LINK: i32 = 0x01;
    /// `LINK_OK` packet type (0x02) - Server → Client
    pub const LINK_OK: i32 = 0x02;
    /// `CAPABILITIES` packet type (0x03) - Client → Server
    pub const CAPABILITIES: i32 = 0x03;
    /// `AUTH_OK` packet type (0x04) - Server → Client
    pub const AUTH_OK: i32 = 0x04;
    /// `CHAT_MESSAGE` packet type (0x05) - Bidirectional
    pub const CHAT_MESSAGE: i32 = 0x05;
    /// `PING` packet type (0x06) - Bidirectional
    pub const PING: i32 = 0x06;
    /// `PONG` packet type (0x07) - Bidirectional
    pub const PONG: i32 = 0x07;
    /// `MODERATION` packet type (0x08) - Server → Client
    pub const MODERATION: i32 = 0x08;
    /// `SYSTEM_DISCONNECT` packet type (0x09) - Server → Client
    pub const SYSTEM_DISCONNECT: i32 = 0x09;
}

/// System disconnect reason codes
pub mod system_disconnect_reason {
    /// Server shutdown
    pub const SHUTDOWN: i32 = 0;
    /// Server maintenance
    pub const MAINTENANCE: i32 = 1;
    /// Internal server error
    pub const INTERNAL_ERROR: i32 = 2;
    /// Server overloaded
    pub const OVERLOADED: i32 = 3;
}

/// Error type for packet validation
#[derive(Debug, Error, Diagnostic)]
pub enum ValidationError {
    /// Invalid message format
    #[error("Invalid message format: {format}")]
    InvalidMessageFormat {
        /// The invalid format value
        format: String,
    },

    /// Invalid UUID format
    #[error("Invalid UUID: {value}")]
    InvalidUuid {
        /// The invalid UUID value
        value: String,
    },

    /// Invalid moderation action value
    #[error("Invalid moderation action: {action} (must be 0-3)")]
    InvalidAction {
        /// The invalid action value
        action: i32,
    },

    /// Invalid moderation scope value
    #[error("Invalid moderation scope: {scope} (must be 0-1)")]
    InvalidScope {
        /// The invalid scope value
        scope: i32,
    },

    /// Message content exceeds maximum size
    #[error("Message too large: {size} bytes (max: {max_size})")]
    MessageTooLarge {
        /// Actual message size in bytes
        size: usize,
        /// Maximum allowed size in bytes
        max_size: usize,
    },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// Chat format constants with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFormat(String);

impl MessageFormat {
    /// Creates a CommonMark message format
    pub fn commonmark() -> Self {
        Self("commonmark".to_string())
    }

    /// Creates a Components (rich text) message format
    pub fn components() -> Self {
        Self("components".to_string())
    }

    /// Creates a MessageFormat from a string value
    /// Returns ValidationError if the value is not "commonmark" or "components"
    pub fn new(value: String) -> Result<Self, ValidationError> {
        match value.as_str() {
            "commonmark" | "components" => Ok(Self(value)),
            _ => Err(ValidationError::InvalidMessageFormat { format: value }),
        }
    }

    /// Returns the string representation of this format
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A MineChat packet with proper type tagging.
///
/// Each variant corresponds to a packet type as defined in the protocol
/// specification (Section 8).
#[derive(Debug, Clone, PartialEq)]
pub enum MineChatPacket {
    /// `LINK` packet (0x01) - Client → Server
    Link {
        /// The linking code from the Minecraft server
        linking_code: LinkCode,
        /// The client UUID identifying this device
        client_uuid: ClientUuid,
    },

    /// `LINK_OK` packet (0x02) - Server → Client
    LinkOk {
        /// The Minecraft UUID of the linked account
        minecraft_uuid: MinecraftUuid,
    },

    /// `CAPABILITIES` packet (0x03) - Client → Server
    Capabilities {
        /// The set of message formats the client supports
        supported_formats: Vec<String>,
        /// The client's preferred format for receiving messages (optional)
        preferred_format: Option<String>,
    },

    /// `AUTH_OK` packet (0x04) - Server → Client
    AuthOk,

    /// `CHAT_MESSAGE` packet (0x05) - Bidirectional
    ChatMessage {
        /// The message format ("commonmark" or "components")
        format: MessageFormat,
        /// The message content
        content: MessageContent,
    },

    /// `PING` packet (0x06) - Bidirectional
    Ping {
        /// Timestamp in milliseconds
        timestamp_ms: i64,
    },

    /// `PONG` packet (0x07) - Bidirectional
    Pong {
        /// Timestamp in milliseconds (should match the corresponding PING)
        timestamp_ms: i64,
    },

    /// `MODERATION` packet (0x08) - Server → Client
    Moderation {
        /// The moderation action (0=warn, 1=mute, 2=kick, 3=ban)
        action: ModerationAction,
        /// The scope of the moderation (0=client, 1=account)
        scope: ModerationScope,
        /// Optional reason for the moderation action
        reason: Option<String>,
        /// Optional duration in seconds (for mute/ban)
        duration_seconds: Option<i32>,
    },

    /// `SYSTEM_DISCONNECT` packet (0x09) - Server → Client
    SystemDisconnect {
        /// The reason code (0=shutdown, 1=maintenance, 2=internal_error, 3=overloaded)
        reason_code: i32,
        /// Human-readable message describing the disconnect
        message: String,
    },
}

impl MineChatPacket {
    /// Returns the packet type identifier for this packet.
    pub fn packet_type(&self) -> i32 {
        let packet_type = match self {
            MineChatPacket::Link { .. } => packet_type::LINK,
            MineChatPacket::LinkOk { .. } => packet_type::LINK_OK,
            MineChatPacket::Capabilities { .. } => packet_type::CAPABILITIES,
            MineChatPacket::AuthOk => packet_type::AUTH_OK,
            MineChatPacket::ChatMessage { .. } => packet_type::CHAT_MESSAGE,
            MineChatPacket::Ping { .. } => packet_type::PING,
            MineChatPacket::Pong { .. } => packet_type::PONG,
            MineChatPacket::Moderation { .. } => packet_type::MODERATION,
            MineChatPacket::SystemDisconnect { .. } => packet_type::SYSTEM_DISCONNECT,
        };
        debug_assert!(
            (0x01..=0x09).contains(&packet_type),
            "Packet type must be 0x01-0x09, got {:#04x}",
            packet_type
        );
        packet_type
    }

    /// Serializes this packet to CBOR bytes with spec-compliant integer keys.
    ///
    /// This method should be used for network transmission to ensure
    /// spec-compliant serialization.
    pub fn to_bytes(&self) -> Result<Vec<u8>, ValidationError> {
        let (packet_type, payload_bytes) = self.serialize_payload()?;

        // Build the envelope with integer keys
        // { 0: packet_type, 1: payload }
        let mut buf = Vec::new();

        // Map header (2 entries)
        buf.push(0xa2);

        // Key 0: packet_type
        buf.push(0x00); // unsigned(0)
                        // Value: packet_type (variable length encoding)
        encode_varint(&mut buf, packet_type as i64);

        // Key 1: payload
        buf.push(0x01); // unsigned(1)
                        // Value: payload bytes
        buf.extend_from_slice(&payload_bytes);

        // Protocol invariants: verify envelope structure
        debug_assert_eq!(buf[0], 0xa2, "Envelope must be map(2)");
        debug_assert_eq!(buf[1], 0x00, "Key 0 must be integer 0");
        debug_assert_eq!(buf[3], 0x01, "Key 1 must be integer 1");
        debug_assert!(
            Self::from_bytes(&buf).is_ok(),
            "Roundtrip serialization failed"
        );

        Ok(buf)
    }

    /// Deserializes a packet from CBOR bytes.
    ///
    /// This method accepts both integer keys (spec-compliant) and string keys
    /// for robustness during migration.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ValidationError> {
        cbor::deserialize_envelope(bytes)
            .map_err(|e| ValidationError::Deserialization(e.to_string()))
            .and_then(|(packet_type, payload)| Self::from_payload(packet_type, payload))
    }

    /// Serializes the payload portion and returns (packet_type, payload_bytes).
    fn serialize_payload(&self) -> Result<(i32, Vec<u8>), ValidationError> {
        let packet_type = self.packet_type();

        let payload_bytes = match self {
            MineChatPacket::Link {
                linking_code,
                client_uuid,
            } => {
                let payload = LinkPayload {
                    linking_code: linking_code.as_str().to_string(),
                    client_uuid: client_uuid.as_str().to_string(),
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::LinkOk { minecraft_uuid } => {
                let payload = LinkOkPayload {
                    minecraft_uuid: minecraft_uuid.as_str().to_string(),
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::Capabilities {
                supported_formats,
                preferred_format,
            } => {
                let payload = CapabilitiesPayload {
                    supported_formats: supported_formats.clone(),
                    preferred_format: preferred_format.clone(),
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::AuthOk => {
                // Empty payload
                vec![0xa0] // empty map
            }
            MineChatPacket::ChatMessage { format, content } => {
                let payload = ChatMessagePayload {
                    format: format.as_str().to_string(),
                    content: content.to_cbor_string(),
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::Ping { timestamp_ms } => {
                let payload = PingPayload {
                    timestamp_ms: *timestamp_ms,
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::Pong { timestamp_ms } => {
                let payload = PongPayload {
                    timestamp_ms: *timestamp_ms,
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::Moderation {
                action,
                scope,
                reason,
                duration_seconds,
            } => {
                let payload = ModerationPayload {
                    action: action.value(),
                    scope: scope.value(),
                    reason: reason.clone(),
                    duration_seconds: *duration_seconds,
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
            MineChatPacket::SystemDisconnect {
                reason_code,
                message,
            } => {
                let payload = SystemDisconnectPayload {
                    reason_code: *reason_code,
                    message: message.clone(),
                };
                cbor::serialize(&payload)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?
            }
        };

        Ok((packet_type, payload_bytes))
    }

    /// Deserializes from a packet type and raw payload bytes.
    fn from_payload(packet_type: i32, payload: serde_cbor::Value) -> Result<Self, ValidationError> {
        debug_assert!(
            (0x01..=0x09).contains(&packet_type),
            "Packet type must be 0x01-0x09, got {:#04x}",
            packet_type
        );
        match packet_type {
            packet_type::LINK => {
                let payload: LinkPayload = decode_payload(payload)?;
                Ok(MineChatPacket::Link {
                    linking_code: LinkCode::new(payload.linking_code)
                        .map_err(|e| ValidationError::Serialization(e.to_string()))?,
                    client_uuid: ClientUuid::new(payload.client_uuid)
                        .map_err(|e| ValidationError::Serialization(e.to_string()))?,
                })
            }
            packet_type::LINK_OK => {
                let payload: LinkOkPayload = decode_payload(payload)?;
                Ok(MineChatPacket::LinkOk {
                    minecraft_uuid: MinecraftUuid::new(payload.minecraft_uuid)
                        .map_err(|e| ValidationError::Serialization(e.to_string()))?,
                })
            }
            packet_type::CAPABILITIES => {
                let payload: CapabilitiesPayload = decode_payload(payload)?;
                Ok(MineChatPacket::Capabilities {
                    supported_formats: payload.supported_formats,
                    preferred_format: payload.preferred_format,
                })
            }
            packet_type::AUTH_OK => Ok(MineChatPacket::AuthOk),
            packet_type::CHAT_MESSAGE => {
                let payload: ChatMessagePayload = decode_payload(payload)?;
                let format_str = payload.format.clone();
                let format = MessageFormat::new(payload.format)
                    .map_err(|e| ValidationError::Serialization(e.to_string()))?;
                let content = if format_str == "components" {
                    MessageContent::Components(
                        serde_json::from_str(&payload.content).unwrap_or_else(|_| {
                            kyori_component_json::Component::text(&payload.content)
                        }),
                    )
                } else {
                    MessageContent::CommonMark(payload.content)
                };
                Ok(MineChatPacket::ChatMessage { format, content })
            }
            packet_type::PING => {
                let payload: PingPayload = decode_payload(payload)?;
                Ok(MineChatPacket::Ping {
                    timestamp_ms: payload.timestamp_ms,
                })
            }
            packet_type::PONG => {
                let payload: PongPayload = decode_payload(payload)?;
                Ok(MineChatPacket::Pong {
                    timestamp_ms: payload.timestamp_ms,
                })
            }
            packet_type::MODERATION => {
                let payload: ModerationPayload = decode_payload(payload)?;
                Ok(MineChatPacket::Moderation {
                    action: ModerationAction::new(payload.action)
                        .map_err(|e| ValidationError::Serialization(e.to_string()))?,
                    scope: ModerationScope::new(payload.scope)
                        .map_err(|e| ValidationError::Serialization(e.to_string()))?,
                    reason: payload.reason,
                    duration_seconds: payload.duration_seconds,
                })
            }
            packet_type::SYSTEM_DISCONNECT => {
                let payload: SystemDisconnectPayload = decode_payload(payload)?;
                Ok(MineChatPacket::SystemDisconnect {
                    reason_code: payload.reason_code,
                    message: payload.message,
                })
            }
            _ => Err(ValidationError::Deserialization(format!(
                "unknown packet type: {}",
                packet_type
            ))),
        }
    }
}

/// Decodes a CBOR Value to a typed payload.
fn decode_payload<T: serde::de::DeserializeOwned>(
    value: serde_cbor::Value,
) -> Result<T, ValidationError> {
    let bytes =
        serde_cbor::to_vec(&value).map_err(|e| ValidationError::Deserialization(e.to_string()))?;
    serde_cbor::de::from_slice(&bytes).map_err(|e| ValidationError::Deserialization(e.to_string()))
}

/// Encodes a variable-length integer to CBOR bytes.
fn encode_varint(buf: &mut Vec<u8>, value: i64) {
    if value >= 0 {
        match value {
            0..=23 => buf.push(value as u8),
            24..=255 => {
                buf.push(24);
                buf.push(value as u8);
            }
            256..=65535 => {
                buf.push(25);
                buf.extend_from_slice(&(value as u16).to_be_bytes());
            }
            65536..=4294967295 => {
                buf.push(26);
                buf.extend_from_slice(&(value as u32).to_be_bytes());
            }
            _ => {
                buf.push(27);
                buf.extend_from_slice(&(value as u64).to_be_bytes());
            }
        }
    } else {
        // Negative integers
        let abs_value = -value - 1;
        match abs_value {
            0..=23 => buf.push(0x20 | (abs_value as u8)),
            24..=255 => {
                buf.push(0x38);
                buf.push(abs_value as u8);
            }
            256..=65535 => {
                buf.push(0x39);
                buf.extend_from_slice(&(abs_value as u16).to_be_bytes());
            }
            _ => {
                buf.push(0x3a);
                buf.extend_from_slice(&(abs_value as u32).to_be_bytes());
            }
        }
    }
}

/// Newtype for Link Codes with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkCode(String);

impl LinkCode {
    /// Creates a new LinkCode from a string
    /// Returns ValidationError if the code is too long (empty is allowed for reconnection)
    pub fn new(code: String) -> Result<Self, ValidationError> {
        if code.len() > 100 {
            return Err(ValidationError::MessageTooLarge {
                size: code.len(),
                max_size: 100,
            });
        }
        Ok(Self(code))
    }

    /// Returns the string representation of this link code
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for Client UUIDs with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientUuid(String);

impl ClientUuid {
    /// Creates a new ClientUuid from a string
    /// Returns ValidationError if the string is not a valid UUID
    pub fn new(uuid: String) -> Result<Self, ValidationError> {
        Uuid::parse_str(&uuid).map_err(|_| ValidationError::InvalidUuid {
            value: uuid.clone(),
        })?;
        Ok(Self(uuid))
    }

    /// Generates a new random ClientUuid
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Returns the string representation of this UUID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for Minecraft UUIDs with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MinecraftUuid(String);

impl MinecraftUuid {
    /// Creates a new MinecraftUuid from a string
    /// Returns ValidationError if the string is not a valid UUID
    pub fn new(uuid: String) -> Result<Self, ValidationError> {
        Uuid::parse_str(&uuid).map_err(|_| ValidationError::InvalidUuid {
            value: uuid.clone(),
        })?;
        Ok(Self(uuid))
    }

    /// Returns the string representation of this UUID
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Moderation action with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationAction(i32);

impl ModerationAction {
    /// Warn action (0)
    pub const WARN: Self = Self(0);
    /// Mute action (1)
    pub const MUTE: Self = Self(1);
    /// Kick action (2)
    pub const KICK: Self = Self(2);
    /// Ban action (3)
    pub const BAN: Self = Self(3);

    /// Creates a new ModerationAction from an i32
    /// Returns ValidationError if the value is not 0-3
    pub fn new(action: i32) -> Result<Self, ValidationError> {
        match action {
            0..=3 => Ok(Self(action)),
            _ => Err(ValidationError::InvalidAction { action }),
        }
    }

    /// Returns the integer value of this action
    pub fn value(&self) -> i32 {
        self.0
    }
}

/// Moderation scope with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationScope(i32);

impl ModerationScope {
    /// Client scope (0) - affects only the current device
    pub const CLIENT: Self = Self(0);
    /// Account scope (1) - affects the entire Minecraft account
    pub const ACCOUNT: Self = Self(1);

    /// Creates a new ModerationScope from an i32
    /// Returns ValidationError if the value is not 0-1
    pub fn new(scope: i32) -> Result<Self, ValidationError> {
        match scope {
            0..=1 => Ok(Self(scope)),
            _ => Err(ValidationError::InvalidScope { scope }),
        }
    }

    /// Returns the integer value of this scope
    pub fn value(&self) -> i32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MessageContent;

    #[test]
    fn test_link_packet_serialization() {
        let packet = MineChatPacket::Link {
            linking_code: LinkCode::new("TEST123".to_string()).unwrap(),
            client_uuid: ClientUuid::new("550e8400-e29b-41d4-a716-446655440000".to_string())
                .unwrap(),
        };

        let bytes = packet.to_bytes().unwrap();

        // Verify the packet type is correct
        assert_eq!(packet.packet_type(), packet_type::LINK);

        // Verify we can deserialize it back
        let deserialized = MineChatPacket::from_bytes(&bytes).unwrap();
        assert_eq!(packet, deserialized);

        println!("LINK packet hex: {}", hex::encode(&bytes));
    }

    #[test]
    fn test_chat_message_serialization() {
        let packet = MineChatPacket::ChatMessage {
            format: MessageFormat::commonmark(),
            content: MessageContent::CommonMark("Hello, world!".to_string()),
        };

        let bytes = packet.to_bytes().unwrap();
        let deserialized = MineChatPacket::from_bytes(&bytes).unwrap();
        assert_eq!(packet, deserialized);
    }

    #[test]
    fn test_ping_pong_distinction() {
        let ping = MineChatPacket::Ping {
            timestamp_ms: 12345,
        };
        let pong = MineChatPacket::Pong {
            timestamp_ms: 12345,
        };

        let ping_bytes = ping.to_bytes().unwrap();
        let pong_bytes = pong.to_bytes().unwrap();

        let ping_deserialized = MineChatPacket::from_bytes(&ping_bytes).unwrap();
        let pong_deserialized = MineChatPacket::from_bytes(&pong_bytes).unwrap();

        assert!(matches!(ping_deserialized, MineChatPacket::Ping { .. }));
        assert!(matches!(pong_deserialized, MineChatPacket::Pong { .. }));
    }

    #[test]
    fn test_system_disconnect_serialization() {
        let packet = MineChatPacket::SystemDisconnect {
            reason_code: 0,
            message: "Server shutdown".to_string(),
        };

        let bytes = packet.to_bytes().unwrap();
        let deserialized = MineChatPacket::from_bytes(&bytes).unwrap();

        match deserialized {
            MineChatPacket::SystemDisconnect {
                reason_code,
                message,
            } => {
                assert_eq!(reason_code, 0);
                assert_eq!(message, "Server shutdown");
            }
            _ => panic!("expected SystemDisconnect"),
        }
    }

    #[test]
    fn test_moderation_serialization() {
        let packet = MineChatPacket::Moderation {
            action: ModerationAction::KICK,
            scope: ModerationScope::ACCOUNT,
            reason: Some("Rule violation".to_string()),
            duration_seconds: None,
        };

        let bytes = packet.to_bytes().unwrap();
        let deserialized = MineChatPacket::from_bytes(&bytes).unwrap();

        match deserialized {
            MineChatPacket::Moderation {
                action,
                scope,
                reason,
                duration_seconds,
            } => {
                assert_eq!(action.value(), 2); // KICK
                assert_eq!(scope.value(), 1); // ACCOUNT
                assert_eq!(reason, Some("Rule violation".to_string()));
                assert_eq!(duration_seconds, None);
            }
            _ => panic!("expected Moderation"),
        }
    }

    #[test]
    fn test_auth_ok_serialization() {
        let packet = MineChatPacket::AuthOk;
        let bytes = packet.to_bytes().unwrap();
        let deserialized = MineChatPacket::from_bytes(&bytes).unwrap();
        assert_eq!(packet, deserialized);
    }

    #[test]
    fn test_integer_keys_in_output() {
        let packet = MineChatPacket::Ping {
            timestamp_ms: 12345,
        };
        let bytes = packet.to_bytes().unwrap();

        // Verify that keys are integers (0 and 1), not strings
        // The first bytes should be: a2 00 06 01 ...
        // a2 = map(2), 00 = key 0, 06 = value 6 (PING), 01 = key 1
        assert!(bytes[0] == 0xa2, "Expected map(2) header");
        assert!(bytes[1] == 0x00, "Expected integer key 0");
        assert!(bytes[2] == 0x06, "Expected PING packet type (0x06)");
        assert!(bytes[3] == 0x01, "Expected integer key 1");

        println!("Integer key test passed. Bytes: {:02X?}", &bytes);
    }
}
