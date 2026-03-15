#[cfg(feature = "tokio")]
use crate::packets::MineChatPacket;
#[cfg(feature = "tokio")]
use crate::protocol::{MessageStream, MineChatError};
#[cfg(feature = "tokio")]
use crate::stream::TokioMessageStream;
use std::io::Error;
use tokio::net::TcpStream;
#[cfg(feature = "tls-rustls")]
use base64::Engine;
#[cfg(feature = "tls-rustls")]
use base64::prelude::*;

#[cfg(feature = "tls-native")]
/// A TLS-enabled `MessageStream` implementation using native TLS.
pub struct TlsMessageStream {
    inner: TokioMessageStream<tokio_native_tls::TlsStream<tokio::net::TcpStream>>,
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
        use native_tls::TlsConnector;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_connector = TlsConnector::from(TlsConnector::new().map_err(|e| Error::other(e))?);
        let tls_stream = tokio_native_tls::TlsConnector::from(tls_connector)
            .connect(host, tcp_stream)
            .await
            .map_err(|e| MineChatError::Io(Error::other(e)))?;

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
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, Error as RustlsError, RootCertStore,
    SignatureScheme,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
#[cfg(feature = "tls-rustls")]
use std::sync::Arc;
#[cfg(feature = "tls-rustls")]
use tokio_rustls::TlsConnector as RustlsTlsConnector;
#[cfg(feature = "tls-rustls")]
use tokio_rustls::client::TlsStream as RustlsTlsStream;

#[cfg(feature = "tls-rustls")]
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};

#[cfg(feature = "tls-rustls")]
/// A TLS-enabled `MessageStream` implementation using rustls with certificate pinning support.
pub struct RustlsTlsMessageStream {
    inner: TokioMessageStream<RustlsTlsStream<tokio::net::TcpStream>>,
    /// The server certificate that was verified during connection (for pinning)
    server_cert: Option<CertificateDer<'static>>,
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
        let mut config = ClientConfig::builder()
            .with_root_certificates(RootCertStore::empty())
            .with_no_client_auth();

        // If we have a pinned certificate, configure verification
        if let Some(pinned) = pinned_cert {
            let cert_der = BASE64_STANDARD
                .decode(pinned)
                .map_err(|e| {
                    MineChatError::ConfigError(format!("Invalid base64 certificate: {}", e))
                })?;

            let cert = CertificateDer::from(cert_der);

            // Create a custom verifier that only accepts the pinned certificate
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(PinnedCertVerifier { pinned_cert: cert }));
        } else {
            // First-time linking: accept server's certificate (trust on first use)
            // Per spec section 3.2: "During initial linking, the client stores the server's certificate"
            // This allows self-signed certificates to work without relying on system root CAs
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(TrustOnFirstUseCertVerifier));
        }

        let connector = RustlsTlsConnector::from(Arc::new(config));
        let server_name: ServerName<'static> = ServerName::try_from(host.to_string())
            .map_err(|e| MineChatError::ConfigError(format!("Invalid hostname: {}", e)))?;

        let tcp_stream = TcpStream::connect(addr).await?;
        let tls_stream = connector
            .connect(server_name, tcp_stream)
            .await
            .map_err(|e| MineChatError::Io(Error::other(e)))?;

        // Extract server certificate for pinning
        let server_cert = tls_stream
            .get_ref()
            .1
            .peer_certificates()
            .and_then(|certs| certs.first())
            .cloned()
            .map(CertificateDer::from);

        Ok(Self {
            inner: TokioMessageStream::new(tls_stream),
            server_cert,
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

#[cfg(feature = "tls-rustls")]
impl RustlsTlsMessageStream {
    /// Get the server certificate for pinning purposes
    pub fn server_certificate(&self) -> Option<&CertificateDer<'static>> {
        self.server_cert.as_ref()
    }

    /// Get the server certificate as a base64-encoded string for storage
    pub fn server_certificate_base64(&self) -> Option<String> {
        self.server_cert
            .as_ref()
            .map(|cert| BASE64_STANDARD.encode(cert.as_ref()))
    }
}

/// Certificate verifier that accepts any certificate (trust on first use).
/// This is used for first-time linking when no pinned certificate exists.
/// Per spec section 3.2: "During initial linking, the client stores the server's certificate"
#[cfg(feature = "tls-rustls")]
#[derive(Debug)]
struct TrustOnFirstUseCertVerifier;

#[cfg(feature = "tls-rustls")]
impl ServerCertVerifier for TrustOnFirstUseCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        // Accept any certificate for first-time linking
        // The certificate will be pinned after this for future connections
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

/// Certificate verifier that accepts a pinned certificate.
#[cfg(feature = "tls-rustls")]
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned_cert: CertificateDer<'static>,
}

#[cfg(feature = "tls-rustls")]
impl ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        // If certificate matches pinned cert, accept
        if end_entity.as_ref() == self.pinned_cert.as_ref() {
            return Ok(ServerCertVerified::assertion());
        }

        // Certificate mismatch MUST cause connection failure
        Err(RustlsError::InvalidCertificate(
            CertificateError::ApplicationVerificationFailure,
        ))
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
