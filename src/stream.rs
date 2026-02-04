#[cfg(feature = "tokio")]
use crate::protocol::{MessageStream, MineChatError, MineChatPacket};
#[cfg(feature = "tokio")]
use log::trace;
#[cfg(feature = "tokio")]
use std::io::Cursor;
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

        // Validate sizes are positive (per spec requirement)
        if serialized.len() > i32::MAX as usize {
            return Err(MineChatError::InvalidFrameSize(serialized.len() as i32));
        }
        if compressed.len() > i32::MAX as usize {
            return Err(MineChatError::InvalidFrameSize(compressed.len() as i32));
        }

        trace!("Sending packet to server");
        // Write big-endian signed integers as required by spec
        self.stream.write_i32(serialized.len() as i32).await?;
        self.stream.write_i32(compressed.len() as i32).await?;
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
        // Read big-endian signed integers as required by spec
        let decompressed_len = self.stream.read_i32().await?;
        let compressed_len = self.stream.read_i32().await?;

        // Validate sizes are positive (per spec: negative or zero sizes MUST terminate connection)
        if decompressed_len <= 0 || compressed_len <= 0 {
            return Err(MineChatError::InvalidFrameSize(decompressed_len));
        }

        let mut compressed = vec![0; compressed_len as usize];
        self.stream.read_exact(&mut compressed).await?;

        let decompressed = zstd::stream::decode_all(Cursor::new(compressed))
            .map_err(|e| MineChatError::Zstd(e.to_string()))?;

        // Validate decompressed size matches expected size (per spec requirement)
        if decompressed.len() != decompressed_len as usize {
            return Err(MineChatError::Zstd(
                "Decompressed size mismatch".to_string(),
            ));
        }

        let packet = serde_cbor::from_slice(&decompressed)?;
        Ok(packet)
    }
}
