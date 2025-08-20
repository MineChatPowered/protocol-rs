use crate::protocol::*;
use log::trace;
use uuid::Uuid;
use std::io::Cursor;
use async_trait::async_trait;

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
    /// Sends a `MineChatMessage` over the Tokio stream.
    ///
    /// This method serializes the message to CBOR, compresses it with zstd,
    /// and then writes the decompressed size, compressed size, and compressed
    /// payload to the underlying Tokio stream.
    ///
    /// # Arguments
    ///
    /// * `msg` - A reference to the `MineChatMessage` to send.
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if an I/O error occurs during writing,
    /// or if serialization/compression fails.
    async fn send_message(&mut self, msg: &MineChatMessage) -> Result<(), MineChatError> {
        trace!("Serializing message {:?}", msg);
        let serialized = serde_cbor::to_vec(msg)?;
        let compressed = zstd::encode_all(Cursor::new(&serialized), 0)
            .map_err(|e| MineChatError::Zstd(e.to_string()))?;

        trace!("Sending message to server");
        self.stream.write_u32(serialized.len() as u32).await?;
        self.stream.write_u32(compressed.len() as u32).await?;
        self.stream.write_all(&compressed).await?;
        Ok(())
    }

    /// Receives a `MineChatMessage` from the Tokio stream.
    ///
    /// This method reads the decompressed and compressed sizes from the stream,
    /// reads the compressed payload, decompresses it with zstd, and then
    /// deserializes it from CBOR into a `MineChatMessage`.
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if an I/O error occurs during reading,
    /// or if decompression/deserialization fails.
    async fn receive_message(&mut self) -> Result<MineChatMessage, MineChatError> {
        let _decompressed_len = self.stream.read_u32().await?;
        let compressed_len = self.stream.read_u32().await?;

        let mut compressed = vec![0; compressed_len as usize];
        self.stream.read_exact(&mut compressed).await?;

        let decompressed = zstd::stream::decode_all(Cursor::new(compressed))
            .map_err(|e| MineChatError::Zstd(e.to_string()))?;

        let msg = serde_cbor::from_slice(&decompressed)?;
        Ok(msg)
    }
}

/// Attempts to link with the server using the provided link code.
///
/// This function generates a new client UUID, sends an `AUTH` message to the server
/// with the client UUID and link code, and then waits for an `AUTH_ACK` response.
/// If the authentication is successful, it returns the client UUID.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream implementing `MessageStream`.
/// * `code` - The link code to authenticate with the server. Can be any type that can be
///   converted to a string reference (e.g., `&str`, `String`).
///
/// # Returns
///
/// `Ok(String)` containing the client UUID if linking is successful.
/// `Err(MineChatError)` if authentication fails, an unexpected message is received,
/// or any other error occurs during message sending/receiving.
pub async fn link_with_server(
    message_stream: &mut (dyn MessageStream + Unpin + Send),
    code: impl AsRef<str>,
) -> Result<String, MineChatError> {
    let client_uuid = Uuid::new_v4().to_string();
    let link_code = code.as_ref();

    trace!("Sending auth message to server");
    message_stream
        .send_message(&MineChatMessage::Auth {
            payload: AuthPayload {
                client_uuid: client_uuid.clone(),
                link_code: link_code.to_string(),
            },
        })
        .await?;

    match message_stream.receive_message().await? {
        MineChatMessage::AuthAck { payload } => {
            if payload.status != "success" {
                return Err(MineChatError::AuthFailed(payload.message));
            }
            trace!("Linked successfully: {}", payload.message);
            Ok(client_uuid)
        }
        _ => Err(MineChatError::AuthFailed("Unexpected response".into())),
    }
}
