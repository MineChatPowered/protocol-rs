use crate::protocol::*;
#[cfg(feature = "tokio")]
use async_trait::async_trait;
use log::trace;
#[cfg(feature = "tokio")]
use std::io::Cursor;
use uuid::Uuid;

#[cfg(feature = "tokio")]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// A `MessageStream` implementation for Tokio asynchronous I/O streams.
///
/// This struct wraps any type that implements `tokio::io::AsyncRead` and
/// `tokio::io::AsyncWrite`, providing the `MessageStream` interface for sending
/// and receiving `MineChatMessage`s over Tokio-compatible streams.
#[cfg(feature = "tokio")]
pub struct TokioMessageStream<S> {
    stream: S,
}

#[cfg(feature = "tokio")]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> TokioMessageStream<S> {
    /// Creates a new `TokioMessageStream` from an asynchronous Tokio stream.
    ///
    /// # Arguments
    ///
    /// * `stream` - The underlying Tokio stream (e.g., `tokio::net::TcpStream`).
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> MessageStream for TokioMessageStream<S> {
    /// Sends a `MineChatPacket` over the Tokio stream.
    ///
    /// This method serializes the packet to CBOR, compresses it with zstd,
    /// and then writes the decompressed size, compressed size, and compressed
    /// payload to the underlying Tokio stream.
    ///
    /// # Arguments
    ///
    /// * `packet` - A reference to the `MineChatPacket` to send.
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if an I/O error occurs during writing,
    /// or if serialization/compression fails.
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError> {
        trace!("Serializing packet {:?}", packet);
        let serialized = serde_cbor::to_vec(packet)?;
        let compressed = zstd::encode_all(Cursor::new(&serialized), 0)
            .map_err(|e| MineChatError::Zstd(e.to_string()))?;

        trace!("Sending packet to server");
        self.stream.write_u32(serialized.len() as u32).await?;
        self.stream.write_u32(compressed.len() as u32).await?;
        self.stream.write_all(&compressed).await?;
        Ok(())
    }

    /// Receives a `MineChatPacket` from the Tokio stream.
    ///
    /// This method reads the decompressed and compressed sizes from the stream,
    /// reads the compressed payload, decompresses it with zstd, and then
    /// deserializes it from CBOR into a `MineChatPacket`.
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if an I/O error occurs during reading,
    /// or if decompression/deserialization fails.
    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError> {
        let _decompressed_len = self.stream.read_u32().await?;
        let compressed_len = self.stream.read_u32().await?;

        let mut compressed = vec![0; compressed_len as usize];
        self.stream.read_exact(&mut compressed).await?;

        let decompressed = zstd::stream::decode_all(Cursor::new(compressed))
            .map_err(|e| MineChatError::Zstd(e.to_string()))?;

        let packet = serde_cbor::from_slice(&decompressed)?;
        Ok(packet)
    }
}

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
