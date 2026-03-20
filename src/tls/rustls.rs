//! Rustls TLS implementation with certificate pinning support.
//!
//! This backend uses rustls, a pure Rust TLS library with no native dependencies.

use crate::packets::MineChatPacket;
use crate::protocol::{MessageStream, MineChatError};
#[cfg(feature = "tokio")]
use crate::stream::TokioMessageStream;
use base64::prelude::*;
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, Error as RustlsError, RootCertStore,
    SignatureScheme, pki_types::{CertificateDer, ServerName, UnixTime},
};
use std::io::Error;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector as RustlsTlsConnector;
use tokio_rustls::client::TlsStream as RustlsTlsStream;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};

/// A TLS-enabled `MessageStream` implementation using rustls with certificate pinning support.
#[cfg(feature = "tokio")]
pub struct RustlsTlsMessageStream {
    inner: TokioMessageStream<RustlsTlsStream<tokio::net::TcpStream>>,
    server_cert: Option<CertificateDer<'static>>,
}

#[cfg(feature = "tokio")]
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

        if let Some(pinned) = pinned_cert {
            let cert_der = BASE64_STANDARD.decode(pinned).map_err(|e| {
                MineChatError::ConfigError(format!("Invalid base64 certificate: {}", e))
            })?;

            let cert = CertificateDer::from(cert_der);
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(PinnedCertVerifier { pinned_cert: cert }));
        } else {
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

    /// Get the server certificate for pinning purposes.
    pub fn server_certificate(&self) -> Option<&CertificateDer<'static>> {
        self.server_cert.as_ref()
    }

    /// Get the server certificate as a base64-encoded string for storage.
    pub fn server_certificate_base64(&self) -> Option<String> {
        self.server_cert
            .as_ref()
            .map(|cert| BASE64_STANDARD.encode(cert.as_ref()))
    }
}

#[cfg(feature = "tokio")]
#[async_trait::async_trait]
impl MessageStream for RustlsTlsMessageStream {
    async fn send_packet(&mut self, packet: &MineChatPacket) -> Result<(), MineChatError> {
        self.inner.send_packet(packet).await
    }

    async fn receive_packet(&mut self) -> Result<MineChatPacket, MineChatError> {
        self.inner.receive_packet().await
    }
}

/// Certificate verifier that accepts any certificate (trust on first use).
/// This is used for first-time linking when no pinned certificate exists.
/// Per spec section 3.2: "During initial linking, the client stores the server's certificate"
#[derive(Debug)]
struct TrustOnFirstUseCertVerifier;

impl ServerCertVerifier for TrustOnFirstUseCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
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
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned_cert: CertificateDer<'static>,
}

impl ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        if end_entity.as_ref() == self.pinned_cert.as_ref() {
            return Ok(ServerCertVerified::assertion());
        }

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
