//! Client-side protocol operations.

use crate::packets::{ClientUuid, LinkCode, MessageFormat, MineChatPacket};
use crate::protocol::{MessageStream, MineChatError};
use crate::types::MessageContent;
use log::trace;
use uuid::Uuid;

/// Attempts to link with the server using the provided link code.
///
/// This function sends a `LINK` message to the server with the client UUID and link code,
/// and then waits for a `LINK_OK` response.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `client_uuid` - The client UUID. If None, a new UUID will be generated (for new linking).
/// * `code` - The link code to authenticate with the server. Empty string for reconnection.
///
/// # Returns
///
/// `Ok((String, String))` containing (client_uuid, minecraft_uuid) if linking is successful.
/// `Err(MineChatError)` if authentication fails, an unexpected message is received,
/// or any other error occurs during packet sending/receiving.
pub async fn link_with_server(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    client_uuid: Option<String>,
    code: impl AsRef<str>,
) -> Result<(String, String), MineChatError> {
    let link_code = code.as_ref();

    let client_uuid_str = match client_uuid {
        Some(uuid) => uuid,
        None if link_code.is_empty() => {
            return Err(MineChatError::AuthFailed(
                "Reconnection requires a client UUID".to_string(),
            ));
        }
        None => Uuid::new_v4().to_string(),
    };

    trace!("Sending LINK packet to server");
    let link_packet = MineChatPacket::Link {
        linking_code: LinkCode::new(link_code.to_string())
            .map_err(|e| MineChatError::ConfigError(format!("invalid link code: {}", e)))?,
        client_uuid: ClientUuid::new(client_uuid_str.clone())
            .map_err(|e| MineChatError::ConfigError(format!("invalid client UUID: {}", e)))?,
    };

    message_stream.send_packet(&link_packet).await?;

    match message_stream.receive_packet().await? {
        MineChatPacket::LinkOk { minecraft_uuid } => {
            trace!(
                "Linked successfully with Minecraft UUID: {}",
                minecraft_uuid.as_str()
            );
            Ok((client_uuid_str, minecraft_uuid.as_str().to_string()))
        }
        _ => Err(MineChatError::AuthFailed("Unexpected response".into())),
    }
}

/// Waits for and validates an AUTH_OK packet from the server.
///
/// This should be called after sending CAPABILITIES to complete the authentication flow.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
///
/// # Returns
///
/// `Ok(())` if AUTH_OK was received successfully.
/// `Err(MineChatError)` if authentication fails or times out.
pub async fn wait_auth_ok(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
) -> Result<(), MineChatError> {
    trace!("Waiting for AUTH_OK packet");
    match message_stream.receive_packet().await? {
        MineChatPacket::AuthOk => {
            trace!("Received AUTH_OK, authentication complete");
            Ok(())
        }
        _ => Err(MineChatError::AuthFailed("Expected AUTH_OK".into())),
    }
}

/// Sends a CAPABILITIES packet to the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `supported_formats` - The set of message formats the client supports (must include "components").
/// * `preferred_format` - The client's preferred format for receiving messages (optional).
///
/// # Errors
///
/// Returns a `MineChatError` if sending the packet fails.
pub async fn send_capabilities(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    supported_formats: Vec<String>,
    preferred_format: Option<String>,
) -> Result<(), MineChatError> {
    trace!("Sending CAPABILITIES packet with formats: {:?}", supported_formats);
    let capabilities_packet = MineChatPacket::Capabilities {
        supported_formats,
        preferred_format,
    };

    message_stream.send_packet(&capabilities_packet).await
}

/// Sends a CHAT_MESSAGE packet to the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `format` - The message format ("commonmark" or "components").
/// * `content` - The message content.
///
/// # Errors
///
/// Returns a `MineChatError` if sending the packet fails.
pub async fn send_chat_message(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    format: &str,
    content: &str,
) -> Result<(), MineChatError> {
    trace!("Sending CHAT_MESSAGE packet: {}", content);
    let chat_packet = MineChatPacket::ChatMessage {
        format: MessageFormat::new(format.to_string())
            .map_err(|e| MineChatError::ConfigError(format!("invalid format: {}", e)))?,
        content: MessageContent::CommonMark(content.to_string()),
    };

    message_stream.send_packet(&chat_packet).await
}

/// Sends a PONG packet to the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `timestamp_ms` - The timestamp from the original PING packet.
///
/// # Errors
///
/// Returns a `MineChatError` if sending the packet fails.
pub async fn send_pong(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    timestamp_ms: i64,
) -> Result<(), MineChatError> {
    trace!("Sending PONG packet: {}", timestamp_ms);
    let pong_packet = MineChatPacket::Pong { timestamp_ms };

    message_stream.send_packet(&pong_packet).await
}
