//! TLS certificate helpers for the QUIC transport.

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::error::Error;

/// Generate a self-signed certificate for use with the QUIC server endpoint.
/// Returns `(cert_chain, private_key)` ready to pass to `rustls::ServerConfig`.
pub fn generate_self_signed(
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Error> {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .map_err(|e| Error::Config(format!("rcgen: {e}")))?;

    let cert_der = cert.der().clone();
    let key_der = PrivatePkcs8KeyDer::from(key_pair.serialize_der());

    Ok((vec![cert_der], key_der.into()))
}

/// Load a certificate and private key from DER-encoded files.
pub fn load_cert_and_key(
    cert_path: &str,
    key_path: &str,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Error> {
    let cert_bytes = std::fs::read(cert_path)
        .map_err(|e| Error::Config(format!("failed to read cert file '{cert_path}': {e}")))?;
    let key_bytes = std::fs::read(key_path)
        .map_err(|e| Error::Config(format!("failed to read key file '{key_path}': {e}")))?;

    let cert = CertificateDer::from(cert_bytes);
    let key = PrivatePkcs8KeyDer::from(key_bytes);

    Ok((vec![cert], key.into()))
}
