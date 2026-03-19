use crate::types::MessageContent;
use miette::Diagnostic;
use serde::de::Error as SerdeDeError;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// A MineChat packet with proper type tagging to fix deserialization ambiguity
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
        /// Whether the client supports rich text components
        supports_components: bool,
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

impl Serialize for MineChatPacket {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
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

        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry(&0, &packet_type)?;

        map.serialize_entry(&1, &Payload::from_packet(self))?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for MineChatPacket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Envelope {
            #[serde(rename = "0")]
            packet_type: i32,
            #[serde(rename = "1")]
            payload: Payload,
        }

        let envelope = Envelope::deserialize(deserializer)?;
        envelope.payload.to_packet(envelope.packet_type)
    }
}

/// Internal payload representation for CBOR serialization (uses string keys per spec)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Payload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    linking_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    minecraft_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supports_components: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timestamp_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    action: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration_seconds: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason_code: Option<i32>,
}

impl Payload {
    /// Convert from a MineChatPacket to a serializable Payload
    pub fn from_packet(packet: &MineChatPacket) -> Self {
        match packet {
            MineChatPacket::Link {
                linking_code,
                client_uuid,
            } => Payload {
                linking_code: Some(linking_code.as_str().to_string()),
                client_uuid: Some(client_uuid.as_str().to_string()),
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::LinkOk { minecraft_uuid } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: Some(minecraft_uuid.as_str().to_string()),
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::Capabilities {
                supports_components,
            } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: Some(*supports_components),
                format: None,
                content: None,
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::AuthOk => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::ChatMessage { format, content } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: Some(format.as_str().to_string()),
                content: Some(content.to_cbor_string()),
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::Ping { timestamp_ms } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: Some(*timestamp_ms),
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::Pong { timestamp_ms } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: Some(*timestamp_ms),
                action: None,
                scope: None,
                reason: None,
                duration_seconds: None,
                reason_code: None,
            },
            MineChatPacket::Moderation {
                action,
                scope,
                reason,
                duration_seconds,
            } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: None,
                action: Some(action.value()),
                scope: Some(scope.value()),
                reason: reason.clone(),
                duration_seconds: *duration_seconds,
                reason_code: None,
            },
            MineChatPacket::SystemDisconnect {
                reason_code,
                message,
            } => Payload {
                linking_code: None,
                client_uuid: None,
                minecraft_uuid: None,
                supports_components: None,
                format: None,
                content: None,
                timestamp_ms: None,
                action: None,
                scope: None,
                reason: Some(message.clone()),
                duration_seconds: None,
                reason_code: Some(*reason_code),
            },
        }
    }

    /// Convert from a Payload to a MineChatPacket using packet_type context
    pub fn to_packet<E>(self, packet_type: i32) -> Result<MineChatPacket, E>
    where
        E: SerdeDeError,
    {
        match packet_type {
            _ if packet_type == packet_type::LINK => {
                let linking_code = self
                    .linking_code
                    .ok_or_else(|| SerdeDeError::missing_field("linking_code"))?;
                let client_uuid = self
                    .client_uuid
                    .ok_or_else(|| SerdeDeError::missing_field("client_uuid"))?;
                Ok(MineChatPacket::Link {
                    linking_code: LinkCode::new(linking_code).map_err(|e| {
                        SerdeDeError::custom(format!("invalid linking_code: {}", e))
                    })?,
                    client_uuid: ClientUuid::new(client_uuid)
                        .map_err(|e| SerdeDeError::custom(format!("invalid client_uuid: {}", e)))?,
                })
            }
            _ if packet_type == packet_type::LINK_OK => {
                let minecraft_uuid = self
                    .minecraft_uuid
                    .ok_or_else(|| SerdeDeError::missing_field("minecraft_uuid"))?;
                Ok(MineChatPacket::LinkOk {
                    minecraft_uuid: MinecraftUuid::new(minecraft_uuid).map_err(|e| {
                        SerdeDeError::custom(format!("invalid minecraft_uuid: {}", e))
                    })?,
                })
            }
            _ if packet_type == packet_type::CAPABILITIES => {
                let supports_components = self.supports_components.unwrap_or(false);
                Ok(MineChatPacket::Capabilities {
                    supports_components,
                })
            }
            _ if packet_type == packet_type::AUTH_OK => Ok(MineChatPacket::AuthOk),
            _ if packet_type == packet_type::CHAT_MESSAGE => {
                let format = self
                    .format
                    .ok_or_else(|| SerdeDeError::missing_field("format"))?;
                let content = self
                    .content
                    .ok_or_else(|| SerdeDeError::missing_field("content"))?;
                let format_str = format.clone();
                Ok(MineChatPacket::ChatMessage {
                    format: MessageFormat::new(format)
                        .map_err(|e| SerdeDeError::custom(format!("invalid format: {}", e)))?,
                    content: if format_str == "components" {
                        MessageContent::Components(
                            serde_json::from_str(&content)
                                .unwrap_or_else(|_| kyori_component_json::Component::text(content)),
                        )
                    } else {
                        MessageContent::CommonMark(content)
                    },
                })
            }
            _ if packet_type == packet_type::PING => {
                let timestamp_ms = self.timestamp_ms.unwrap_or(0);
                Ok(MineChatPacket::Ping { timestamp_ms })
            }
            _ if packet_type == packet_type::PONG => {
                let timestamp_ms = self.timestamp_ms.unwrap_or(0);
                Ok(MineChatPacket::Pong { timestamp_ms })
            }
            _ if packet_type == packet_type::MODERATION => {
                let action = self
                    .action
                    .ok_or_else(|| SerdeDeError::missing_field("action"))?;
                let scope = self
                    .scope
                    .ok_or_else(|| SerdeDeError::missing_field("scope"))?;
                Ok(MineChatPacket::Moderation {
                    action: ModerationAction::new(action)
                        .map_err(|e| SerdeDeError::custom(format!("invalid action: {}", e)))?,
                    scope: ModerationScope::new(scope)
                        .map_err(|e| SerdeDeError::custom(format!("invalid scope: {}", e)))?,
                    reason: self.reason,
                    duration_seconds: self.duration_seconds,
                })
            }
            _ if packet_type == packet_type::SYSTEM_DISCONNECT => {
                let reason_code = self.reason_code.unwrap_or(0);
                let message = self.reason.unwrap_or_default();
                Ok(MineChatPacket::SystemDisconnect {
                    reason_code,
                    message,
                })
            }
            _ => Err(SerdeDeError::custom(format!(
                "unknown packet type: {}",
                packet_type
            ))),
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
        // Validate UUID format
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
        // Validate UUID format
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
    use kyori_component_json::{Color, Component, NamedColor};

    #[test]
    fn test_packet_serialization_roundtrip() {
        let original = MineChatPacket::ChatMessage {
            format: MessageFormat::commonmark(),
            content: MessageContent::CommonMark("Hello, world!".to_string()),
        };

        let serialized = serde_cbor::to_vec(&original).unwrap();
        let deserialized: MineChatPacket = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_ping_pong_distinction() {
        let ping = MineChatPacket::Ping {
            timestamp_ms: 12345,
        };
        let pong = MineChatPacket::Pong {
            timestamp_ms: 12345,
        };

        let ping_serialized = serde_cbor::to_vec(&ping).unwrap();
        let pong_serialized = serde_cbor::to_vec(&pong).unwrap();

        let ping_deserialized: MineChatPacket = serde_cbor::from_slice(&ping_serialized).unwrap();
        let pong_deserialized: MineChatPacket = serde_cbor::from_slice(&pong_serialized).unwrap();

        assert!(matches!(ping_deserialized, MineChatPacket::Ping { .. }));
        assert!(matches!(pong_deserialized, MineChatPacket::Pong { .. }));
    }

    #[test]
    fn test_component_message_serialization() {
        let component = Component::text("Hello")
            .color(Some(Color::Named(NamedColor::Red)))
            .decoration(kyori_component_json::TextDecoration::Bold, Some(true));

        let original = MineChatPacket::ChatMessage {
            format: MessageFormat::components(),
            content: MessageContent::Components(component),
        };

        let serialized = serde_cbor::to_vec(&original).unwrap();
        let deserialized: MineChatPacket = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(original, deserialized);
    }
}
