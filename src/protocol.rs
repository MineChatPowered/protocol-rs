use async_trait::async_trait;
use kyori_component_json::Component;
use log::error;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io;

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
}

#[async_trait]
pub trait MessageStream {
    async fn send_message(&mut self, msg: &MineChatMessage) -> Result<(), MineChatError>;
    async fn receive_message(&mut self) -> Result<MineChatMessage, MineChatError>;
}

/// The different types of messages that can be sent and received in the MineChat protocol.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MineChatMessage {
    /// An authentication message, containing the client's UUID and link code.
    #[serde(rename = "AUTH")]
    Auth { payload: AuthPayload },

    /// An acknowledgment of a successful authentication, containing the server's response.
    #[serde(rename = "AUTH_ACK")]
    AuthAck { payload: AuthAckPayload },

    /// A chat message, containing the message text.
    #[serde(rename = "CHAT")]
    Chat { payload: ChatPayload },

    /// A broadcast message, containing the message text and the sender's name.
    #[serde(rename = "BROADCAST")]
    Broadcast { payload: BroadcastPayload },

    /// A disconnect message, containing the reason for the disconnection.
    #[serde(rename = "DISCONNECT")]
    Disconnect { payload: DisconnectPayload },
}

/// The payload for an authentication message.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthPayload {
    /// The client's UUID.
    pub client_uuid: String,
    /// The link code used to authenticate with the server.
    pub link_code: String,
}

/// The payload for an authentication acknowledgment message.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthAckPayload {
    /// The status of the authentication (either "success" or "failure").
    pub status: String,
    /// A message describing the authentication status.
    pub message: String,
    /// The client's Minecraft UUID, if available.
    pub minecraft_uuid: Option<String>,
    /// The client's Minecraft username, if available.
    pub username: Option<String>,
}

/// The payload for a chat message.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatPayload {
    /// The text of the chat message.
    pub message: Component,
}

/// The payload for a broadcast message.
#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastPayload {
    /// The name of the sender.
    pub from: String,
    /// The text of the broadcast message.
    pub message: Component,
}

/// The payload for a disconnect message.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct DisconnectPayload {
    /// The reason for the disconnection.
    pub reason: String,
}


#[cfg(test)]
mod tests {
    use super::*;
    use kyori_component_json::Component;

    #[test]
    fn test_auth_message_serialization() {
        let msg = MineChatMessage::Auth {
            payload: AuthPayload {
                client_uuid: "test-uuid".to_string(),
                link_code: "test-code".to_string(),
            },
        };
        let serialized = serde_cbor::to_vec(&msg).unwrap();
        let deserialized: MineChatMessage = serde_cbor::from_slice(&serialized).unwrap();
        
        if let MineChatMessage::Auth { payload } = deserialized {
            assert_eq!(payload.client_uuid, "test-uuid");
            assert_eq!(payload.link_code, "test-code");
        } else {
            panic!("Deserialized message is not an Auth message");
        }
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = MineChatMessage::Chat {
            payload: ChatPayload {
                message: Component::text("Hello, world!"),
            },
        };
        let serialized = serde_cbor::to_vec(&msg).unwrap();
        let deserialized: MineChatMessage = serde_cbor::from_slice(&serialized).unwrap();

        if let MineChatMessage::Chat { payload } = deserialized {
            if let Component::Object(obj) = payload.message {
                assert_eq!(obj.text, Some("Hello, world!".to_string()));
            } else {
                panic!("Expected Component::Object");
            }
        } else {
            panic!("Deserialized message is not a Chat message");
        }
    }
}