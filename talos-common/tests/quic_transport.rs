//! Integration tests for the QUIC transport layer.
//!
//! These tests require the `quic` feature:
//!   cargo test -p talos-common --features quic --test quic_transport

#![cfg(feature = "quic")]

use std::time::Duration;

use talos_common::config::QuicTransportConfig;
use talos_common::transport::quic::QuicTransport;

/// Spin up a QUIC server endpoint on an ephemeral port and connect a client to it.
/// Verifies that the TLS handshake succeeds with a self-signed certificate.
#[tokio::test]
async fn quic_connect_accept_self_signed() {
    let config = QuicTransportConfig {
        bind_addr: "127.0.0.1:0".to_string(), // OS assigns ephemeral port
        cert_path: None,
        key_path: None,
    };

    let server_endpoint = QuicTransport::bind(&config).await.unwrap();
    let server_addr = server_endpoint.local_addr().unwrap();

    // Accept connection in background
    let accept_task = tokio::spawn(async move {
        let incoming = server_endpoint.accept().await.unwrap();
        incoming.await.unwrap()
    });

    // Give the server time to start listening
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Connect client (skips TLS verification)
    let client_conn = QuicTransport::connect(server_addr).await.unwrap();

    // Ensure the server accepted the connection
    let server_conn = tokio::time::timeout(Duration::from_secs(2), accept_task)
        .await
        .expect("timed out")
        .expect("accept task panicked");

    // Verify both sides see a stable connection
    assert!(!client_conn.stable_id().to_string().is_empty());
    assert!(!server_conn.stable_id().to_string().is_empty());

    client_conn.close(0u32.into(), b"done");
}
