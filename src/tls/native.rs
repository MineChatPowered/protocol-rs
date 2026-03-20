//! Native TLS implementation using native-tls (OpenSSL).
//!
//! This backend uses the system's native TLS library (typically OpenSSL).

use crate::packets::MineChatPacket;
use crate::protocol::{MessageStream, MineChatError};
#[cfg(feature = "tokio")]
use crate::stream::TokioMessageStream;
use std::io::Error;
use tokio::net::TcpStream;

/// A TLS-enabled `MessageStream` implementation using native TLS.
#[cfg(feature = "tokio")]
pub struct TlsMessageStream {
    inner: TokioMessageStream<tokio_native_tls::TlsStream<tokio::net::TcpStream>>,
}

#[cfg(feature = "tokio")]
impl TlsMessageStream {
    /// Creates a new `TlsMessageStream` by connecting to the server with TLS.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname to connect to and verify in the certificate
    /// * `addr` - The socket address to connect to
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if TLS connection fails.
    pub async fn connect(host: &str, addr: &str) -> Result<Self, MineChatError> {
        use native_tls::TlsConnector;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_connector = TlsConnector::new().map_err(Error::other)?;
        let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
            .connect(host, tcp_stream)
            .await
            .map_err(|e| MineChatError::Io(Error::other(e)))?;

        Ok(Self {
            inner: TokioMessageStream::new(tls_stream),
        })
    }
}

#[cfg(feature = "tokio")]
#[async_trait::async_trait]
impl MessageStream for TlsMessageStream {
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError> {
        self.inner.send_packet(packet).await
    }

    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError> {
        self.inner.receive_packet().await
    }
}
