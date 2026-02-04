use crate::protocol::{
    CapabilitiesPayload, ChatMessagePayload, DisconnectPayload, LinkPayload, MessageStream,
    MineChatError, MineChatPacket, Payload, PongPayload, packet_types,
};
use log::trace;
use uuid::Uuid;

/// Attempts to link with the server using the provided link code.
///
/// This function generates a new client UUID (if reconnection), sends a `LINK` message to the server
/// with the client UUID and link code, and then waits for a `LINK_OK` response.
/// If the linking is successful, it returns the client UUID and Minecraft UUID.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `code` - The link code to authenticate with the server. Empty string for reconnection.
///
/// # Returns
///
/// `Ok((String, String))` containing (client_uuid, minecraft_uuid) if linking is successful.
/// `Err(MineChatError)` if authentication fails, an unexpected message is received,
/// or any other error occurs during packet sending/receiving.
pub async fn link_with_server(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    code: impl AsRef<str>,
) -> Result<(String, String), MineChatError> {
    let link_code = code.as_ref();

    // If code is empty (reconnection), we need to load the existing client UUID
    // For now, generate a new one if code is not empty
    let client_uuid = if link_code.is_empty() {
        // In a real implementation, this would load the existing client UUID
        // For now, we'll use a placeholder that the server should handle
        "reconnection-placeholder".to_string()
    } else {
        Uuid::new_v4().to_string()
    };

    trace!("Sending LINK packet to server");
    let link_packet = MineChatPacket {
        packet_type: packet_types::LINK,
        payload: Payload::Link(LinkPayload {
            linking_code: link_code.to_string(),
            client_uuid: client_uuid.clone(),
        }),
    };

    message_stream.send_packet(&link_packet).await?;

    match message_stream.receive_packet().await? {
        MineChatPacket {
            packet_type: packet_types::LINK_OK,
            payload: Payload::LinkOk(payload),
        } => {
            trace!(
                "Linked successfully with Minecraft UUID: {}",
                payload.minecraft_uuid
            );
            Ok((client_uuid, payload.minecraft_uuid))
        }
        _ => Err(MineChatError::AuthFailed("Unexpected response".into())),
    }
}

/// Sends a CAPABILITIES packet to the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `supports_components` - Whether the client supports Minecraft text components.
///
/// # Errors
///
/// Returns a `MineChatError` if sending the packet fails.
pub async fn send_capabilities(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    supports_components: bool,
) -> Result<(), MineChatError> {
    trace!("Sending CAPABILITIES packet");
    let capabilities_packet = MineChatPacket {
        packet_type: packet_types::CAPABILITIES,
        payload: Payload::Capabilities(CapabilitiesPayload {
            supports_components,
        }),
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
    let chat_packet = MineChatPacket {
        packet_type: packet_types::CHAT_MESSAGE,
        payload: Payload::ChatMessage(ChatMessagePayload {
            format: format.to_string(),
            content: content.to_string(),
        }),
    };

    message_stream.send_packet(&chat_packet).await
}

/// Sends a DISCONNECT packet to the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `reason` - The reason for disconnection.
///
/// # Errors
///
/// Returns a `MineChatError` if sending the packet fails.
pub async fn send_disconnect(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    reason: &str,
) -> Result<(), MineChatError> {
    trace!("Sending DISCONNECT packet: {}", reason);
    let disconnect_packet = MineChatPacket {
        packet_type: packet_types::DISCONNECT,
        payload: Payload::Disconnect(DisconnectPayload {
            reason: reason.to_string(),
        }),
    };

    message_stream.send_packet(&disconnect_packet).await
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
    let pong_packet = MineChatPacket {
        packet_type: packet_types::PONG,
        payload: Payload::Pong(PongPayload { timestamp_ms }),
    };

    message_stream.send_packet(&pong_packet).await
}
