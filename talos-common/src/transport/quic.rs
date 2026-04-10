//! QUIC transport implementation using `quinn` and `rustls`.

use std::sync::Arc;

use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig, VarInt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

use crate::config::QuicTransportConfig;
use crate::error::Error;

use super::certs;

/// Helper providing server and client QUIC endpoint construction.
pub struct QuicTransport;

/// Install the `ring` crypto provider as the process-level default for rustls.
/// Idempotent — safe to call multiple times.
fn ensure_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

impl QuicTransport {
    /// Bind a QUIC server endpoint at the address specified in `config`.
    ///
    /// If `cert_path`/`key_path` are specified the certificate is loaded from
    /// disk; otherwise a self-signed certificate is generated in memory.
    pub async fn bind(config: &QuicTransportConfig) -> Result<Endpoint, Error> {
        ensure_crypto_provider();
        let (cert_chain, key): (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) =
            if let (Some(cert_path), Some(key_path)) = (&config.cert_path, &config.key_path) {
                tracing::info!(cert = %cert_path, key = %key_path, "loading QUIC certificates");
                certs::load_cert_and_key(cert_path, key_path)?
            } else {
                tracing::info!("generating self-signed QUIC certificate");
                certs::generate_self_signed()?
            };

        let rustls_server = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| Error::Config(format!("TLS config error: {e}")))?;

        let quic_server = QuicServerConfig::try_from(rustls_server)
            .map_err(|e| Error::Config(format!("QUIC server config error: {e}")))?;

        let server_config = ServerConfig::with_crypto(Arc::new(quic_server));

        let addr: std::net::SocketAddr = config.bind_addr.parse().map_err(|e| {
            Error::Config(format!(
                "invalid QUIC bind address '{}': {e}",
                config.bind_addr
            ))
        })?;

        Endpoint::server(server_config, addr)
            .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::AddrInUse, e.to_string())))
    }

    /// Create a QUIC client endpoint and connect to `addr`.
    ///
    /// Uses a `SkipServerVerification` TLS verifier — suitable for
    /// local-network connections to agents with self-signed certificates.
    pub async fn connect(addr: std::net::SocketAddr) -> Result<quinn::Connection, Error> {
        ensure_crypto_provider();
        let rustls_client = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
            .with_no_client_auth();

        let quic_client = QuicClientConfig::try_from(rustls_client)
            .map_err(|e| Error::Config(format!("QUIC client config error: {e}")))?;

        let mut transport_config = TransportConfig::default();
        transport_config.max_concurrent_uni_streams(VarInt::from_u32(64));

        let mut client_config = ClientConfig::new(Arc::new(quic_client));
        client_config.transport_config(Arc::new(transport_config));

        let local: std::net::SocketAddr = "0.0.0.0:0".parse().unwrap();
        let mut endpoint = Endpoint::client(local).map_err(Error::Io)?;
        endpoint.set_default_client_config(client_config);

        endpoint
            .connect(addr, "localhost")
            .map_err(|e| Error::Config(format!("QUIC connect error: {e}")))?
            .await
            .map_err(|e| Error::Config(format!("QUIC connection error: {e}")))
    }
}

/// A `ServerCertVerifier` that accepts any certificate without validation.
///
/// Suitable for trusted local networks (robot ↔ developer workstation over
/// WiFi/Ethernet).  The upgrade path to real TLS is: swap this verifier.
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
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
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}
