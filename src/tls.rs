#[cfg(feature = "cert-pinning")]
use crate::protocol::MineChatError;
#[cfg(feature = "tokio")]
use crate::stream::TokioMessageStream;
#[cfg(feature = "cert-pinning")]
use base64::Engine;
#[cfg(feature = "cert-pinning")]
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
#[cfg(feature = "cert-pinning")]
use std::sync::Arc;
#[cfg(feature = "tls-native")]
use tokio_native_tls::TlsConnector;
#[cfg(feature = "tls-rustls")]
use tokio_rustls::{TlsConnector, client::TlsStream as RustlsTlsStream};

#[cfg(feature = "tls-native")]
/// A TLS-enabled `MessageStream` implementation using native TLS.
pub struct TlsMessageStream {
    inner: TokioMessageStream<TlsStream<tokio::net::TcpStream>>,
}

#[cfg(feature = "tls-native")]
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
        use native_tls::TlsConnector as NativeTlsConnector;
        use tokio::net::TcpStream;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_connector = TlsConnector::from(NativeTlsConnector::new()?);
        let tls_stream = tls_connector
            .connect(host, tcp_stream)
            .await
            .map_err(|e| MineChatError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(Self {
            inner: TokioMessageStream::new(tls_stream),
        })
    }
}

#[cfg(feature = "tls-native")]
#[async_trait::async_trait]
impl MessageStream for TlsMessageStream {
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError> {
        self.inner.send_packet(packet).await
    }

    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError> {
        self.inner.receive_packet().await
    }
}

#[cfg(feature = "tls-rustls")]
/// A TLS-enabled `MessageStream` implementation using rustls with certificate pinning support.
pub struct RustlsTlsMessageStream {
    inner: TokioMessageStream<RustlsTlsStream<tokio::net::TcpStream>>,
}

#[cfg(feature = "tls-rustls")]
impl RustlsTlsMessageStream {
    /// Creates a new `RustlsTlsMessageStream` with certificate pinning.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname to connect to
    /// * `addr` - The socket address to connect to  
    /// * `pinned_cert` - Optional pinned certificate (base64-encoded DER)
    ///
    /// # Errors
    ///
    /// Returns a `MineChatError` if TLS connection or pinning verification fails.
    pub async fn connect_with_pinning(
        host: &str,
        addr: &str,
        pinned_cert: Option<&str>,
    ) -> Result<Self, MineChatError> {
        use tokio::net::TcpStream;

        let mut config = ClientConfig::builder()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth();

        // If we have a pinned certificate, configure verification
        if let Some(pinned) = pinned_cert {
            let cert_der = base64::prelude::BASE64_STANDARD
                .decode(pinned)
                .map_err(|e| {
                    MineChatError::ConfigError(format!("Invalid base64 certificate: {}", e))
                })?;

            let cert = CertificateDer::from(cert_der);

            // Create a custom verifier that only accepts the pinned certificate
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(PinnedCertVerifier {
                    pinned_cert: cert,
                    allow_rotation: false, // Per spec, certificate mismatch MUST cause connection failure
                }));
        } else {
            // Use system root certificates if no pinning
            let mut root_store = RootCertStore::empty();
            let native_certs = rustls_native_certs::load_native_certs().map_err(|e| {
                MineChatError::ConfigError(format!("Failed to load system certs: {}", e))
            })?;
            root_store.add_parsable_certificates(native_certs);
            config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();
        }

        let connector = TlsConnector::from(Arc::new(config));
        let server_name: ServerName<'static> = ServerName::try_from(host.to_string())
            .map_err(|e| MineChatError::ConfigError(format!("Invalid hostname: {}", e)))?;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_stream = connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|e| MineChatError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(Self {
            inner: TokioMessageStream::new(tls_stream),
        })
    }
}

#[cfg(feature = "tls-rustls")]
#[async_trait::async_trait]
impl MessageStream for RustlsTlsMessageStream {
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError> {
        self.inner.send_packet(packet).await
    }

    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError> {
        self.inner.receive_packet().await
    }
}

#[cfg(feature = "cert-pinning")]
/// Certificate verifier that accepts a pinned certificate.
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned_cert: CertificateDer<'static>,
    allow_rotation: bool,
}

#[cfg(feature = "cert-pinning")]
impl rustls::client::danger::ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        // If certificate matches pinned cert, accept
        if end_entity.as_ref() == self.pinned_cert.as_ref() {
            return Ok(rustls::client::danger::ServerCertVerified::assertion());
        }

        // If rotation is allowed, we might want to log this for admin review
        // But per spec, certificate mismatch MUST cause connection failure
        Err(rustls::Error::InvalidCertificate(
            rustls::CertificateError::ApplicationVerificationFailure,
        ))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}
