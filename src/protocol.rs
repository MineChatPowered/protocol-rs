use async_trait::async_trait;
use kyori_component_json::Component;
use log::error;
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
}

/// A trait for sending and receiving `MineChatMessage`s over an asynchronous stream.
///
/// This trait abstracts over the underlying transport, allowing for different
/// implementations (e.g., Tokio TCP streams, in-memory streams for testing).
#[async_trait]
pub trait MessageStream {
    /// Sends a `MineChatMessage` over the stream.
    ///
    /// The message is serialized using CBOR, compressed with zstd, and then framed
    /// with decompressed and compressed lengths before being written to the stream.
    ///
    /// # Arguments
    ///
    /// * `msg` - A reference to the `MineChatMessage` to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the message was sent successfully, or a `MineChatError` otherwise.
    async fn send_message(&mut self, msg: &MineChatMessage) -> Result<(), MineChatError>;

    /// Receives a `MineChatMessage` from the stream.
    ///
    /// This method reads the message framing (decompressed and compressed lengths),
    /// reads the compressed payload, decompresses it with zstd, and then deserializes
    /// it from CBOR into a `MineChatMessage`.
    ///
    /// # Returns
    ///
    /// `Ok(MineChatMessage)` if a message was received and parsed successfully,
    /// or a `MineChatError` otherwise.
    async fn receive_message(&mut self) -> Result<MineChatMessage, MineChatError>;
}

/// The different types of messages that can be sent and received in the MineChat protocol.
///
/// Each variant represents a distinct message type with its associated payload.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MineChatMessage {
    /// An authentication message, sent by the client to authenticate with the server.
    #[serde(rename = "AUTH")]
    Auth {
        /// The authentication payload.
        payload: AuthPayload,
    },

    /// An acknowledgment of an authentication attempt, sent by the server.
    ///
    /// This message indicates whether the authentication was successful and provides
    /// additional information like the client's Minecraft UUID and username if available.
    #[serde(rename = "AUTH_ACK")]
    AuthAck {
        /// The authentication acknowledgment payload.
        payload: AuthAckPayload,
    },

    /// A chat message, sent by the client to send a message to the server.
    #[serde(rename = "CHAT")]
    Chat {
        /// The chat message payload.
        payload: ChatPayload,
    },

    /// A broadcast message, sent by the server to distribute a chat message to all connected clients.
    #[serde(rename = "BROADCAST")]
    Broadcast {
        /// The broadcast message payload.
        payload: BroadcastPayload,
    },

    /// A disconnect message, sent by either the client or the server to gracefully close the connection.
    #[serde(rename = "DISCONNECT")]
    Disconnect {
        /// The disconnect message payload.
        payload: DisconnectPayload,
    },
}

/// The payload for an authentication message (`MineChatMessage::Auth`).
///
/// Contains the client's unique identifier and a link code for authentication.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthPayload {
    /// The client's unique identifier (UUID).
    pub client_uuid: String,
    /// The link code used to authenticate with the server.
    pub link_code: String,
}

/// The payload for an authentication acknowledgment message (`MineChatMessage::AuthAck`).
///
/// Provides the status of the authentication attempt and optional user details.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct AuthAckPayload {
    /// The status of the authentication (e.g., "success", "failure").
    pub status: String,
    /// A descriptive message regarding the authentication status.
    pub message: String,
    /// The client's Minecraft UUID, if authentication was successful and available.
    pub minecraft_uuid: Option<String>,
    /// The client's Minecraft username, if authentication was successful and available.
    pub username: Option<String>,
}

/// The payload for a chat message (`MineChatMessage::Chat`).
///
/// Contains the rich text component of the message.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatPayload {
    /// The rich text content of the chat message, represented as a `kyori_component_json::Component`.
    pub message: Component,
}

/// The payload for a broadcast message (`MineChatMessage::Broadcast`).
///
/// Contains the sender's name and the rich text content of the broadcast.
#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastPayload {
    /// The name of the sender of the broadcast message.
    pub from: String,
    /// The rich text content of the broadcast message, represented as a `kyori_component_json::Component`.
    pub message: Component,
}

/// The payload for a disconnect message (`MineChatMessage::Disconnect`).
///
/// Provides a reason for the connection termination.
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
