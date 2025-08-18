use crate::protocol::*;
use async_trait::async_trait;
use log::trace;
use std::io::Cursor;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

#[cfg(feature = "tokio")]
pub struct TokioMessageStream<S> {
    stream: S,
}

#[cfg(feature = "tokio")]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> TokioMessageStream<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

#[cfg(feature = "tokio")]
#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> MessageStream for TokioMessageStream<S> {
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

/// Handles linking with the server.
///
/// # Arguments
///
/// * `message_stream` - A mutable reference to a message stream.
/// * `code` - The link code to authenticate with the server.
///
/// # Returns
/// * `Result<String, MineChatError>` - Returns the client UUID if linking is successful, otherwise returns an error.
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
