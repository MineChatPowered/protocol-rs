use crate::types::MessageContent;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Error type for packet validation
#[derive(Debug, Error, Diagnostic)]
pub enum ValidationError {
    #[error("Invalid message format: {format}")]
    InvalidMessageFormat { format: String },

    #[error("Invalid UUID: {value}")]
    InvalidUuid { value: String },

    #[error("Invalid moderation action: {action} (must be 0-3)")]
    InvalidAction { action: i32 },

    #[error("Invalid moderation scope: {scope} (must be 0-1)")]
    InvalidScope { scope: i32 },

    #[error("Message too large: {size} bytes (max: {max_size})")]
    MessageTooLarge { size: usize, max_size: usize },
}

/// Chat format constants with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageFormat(String);

impl MessageFormat {
    pub fn commonmark() -> Self {
        Self("commonmark".to_string())
    }

    pub fn components() -> Self {
        Self("components".to_string())
    }

    pub fn new(value: String) -> Result<Self, ValidationError> {
        match value.as_str() {
            "commonmark" | "components" => Ok(Self(value)),
            _ => Err(ValidationError::InvalidMessageFormat { format: value }),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A MineChat packet with proper type tagging to fix deserialization ambiguity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "packet_type", content = "payload", rename_all = "snake_case")]
pub enum MineChatPacket {
    /// LINK packet (0x01) - Client → Server
    Link {
        linking_code: LinkCode,
        client_uuid: ClientUuid,
    },

    /// LINK_OK packet (0x02) - Server → Client
    LinkOk { minecraft_uuid: MinecraftUuid },

    /// CAPABILITIES packet (0x03) - Client → Server
    Capabilities { supports_components: bool },

    /// AUTH_OK packet (0x04) - Server → Client
    AuthOk,

    /// CHAT_MESSAGE packet (0x05) - Bidirectional
    #[serde(deserialize_with = "deserialize_chat_message")]
    ChatMessage {
        format: MessageFormat,
        content: MessageContent,
    },

    /// PING packet (0x06) - Bidirectional
    Ping { timestamp_ms: i64 },

    /// PONG packet (0x07) - Bidirectional
    Pong { timestamp_ms: i64 },

    /// MODERATION packet (0x08) - Server → Client
    #[serde(deserialize_with = "deserialize_moderation")]
    Moderation {
        action: ModerationAction,
        scope: ModerationScope,
        reason: Option<String>,
        duration_seconds: Option<i32>,
    },

    /// DISCONNECT packet (0x80) - Implementation-private
    Disconnect { reason: String },
}

/// Context-aware deserializer for ChatMessage
fn deserialize_chat_message<'de, D>(
    deserializer: D,
) -> Result<(MessageFormat, MessageContent), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ChatMessageHelper {
        format: MessageFormat,
        content: MessageContent,
    }

    let helper = ChatMessageHelper::deserialize(deserializer)?;
    Ok((helper.format, helper.content))
}

/// Context-aware deserializer for Moderation
fn deserialize_moderation<'de, D>(
    deserializer: D,
) -> Result<
    (
        ModerationAction,
        ModerationScope,
        Option<String>,
        Option<i32>,
    ),
    D::Error,
>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ModerationHelper {
        action: ModerationAction,
        scope: ModerationScope,
        reason: Option<String>,
        duration_seconds: Option<i32>,
    }

    let helper = ModerationHelper::deserialize(deserializer)?;
    Ok((
        helper.action,
        helper.scope,
        helper.reason,
        helper.duration_seconds,
    ))
}

/// Newtype for Link Codes with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkCode(String);

impl LinkCode {
    pub fn new(code: String) -> Result<Self, ValidationError> {
        if code.trim().is_empty() {
            return Err(ValidationError::InvalidMessageFormat {
                format: "empty link code".to_string(),
            });
        }
        if code.len() > 100 {
            return Err(ValidationError::MessageTooLarge {
                size: code.len(),
                max_size: 100,
            });
        }
        Ok(Self(code))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for Client UUIDs with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientUuid(String);

impl ClientUuid {
    pub fn new(uuid: String) -> Result<Self, ValidationError> {
        // Validate UUID format
        Uuid::parse_str(&uuid).map_err(|_| ValidationError::InvalidUuid {
            value: uuid.clone(),
        })?;
        Ok(Self(uuid))
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Newtype for Minecraft UUIDs with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MinecraftUuid(String);

impl MinecraftUuid {
    pub fn new(uuid: String) -> Result<Self, ValidationError> {
        // Validate UUID format
        Uuid::parse_str(&uuid).map_err(|_| ValidationError::InvalidUuid {
            value: uuid.clone(),
        })?;
        Ok(Self(uuid))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Moderation action with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationAction(i32);

impl ModerationAction {
    pub const WARN: Self = Self(0);
    pub const MUTE: Self = Self(1);
    pub const KICK: Self = Self(2);
    pub const BAN: Self = Self(3);

    pub fn new(action: i32) -> Result<Self, ValidationError> {
        match action {
            0..=3 => Ok(Self(action)),
            _ => Err(ValidationError::InvalidAction { action }),
        }
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

/// Moderation scope with validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationScope(i32);

impl ModerationScope {
    pub const CLIENT: Self = Self(0);
    pub const ACCOUNT: Self = Self(1);

    pub fn new(scope: i32) -> Result<Self, ValidationError> {
        match scope {
            0..=1 => Ok(Self(scope)),
            _ => Err(ValidationError::InvalidScope { scope }),
        }
    }

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
