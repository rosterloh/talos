# Transports

Talos supports two transports: Unix domain sockets and QUIC.

## Unix Domain Sockets

UDS is the default local transport. It is appropriate when the client and agent
run on the same machine or inside an environment where the Unix socket path is
shared.

The default socket path is:

```text
/tmp/talos.sock
```

When the agent starts, it creates the socket listener. If a stale socket file is
left behind from an earlier run, the UDS transport removes it before binding.

## QUIC

QUIC is for remote observation and control across a network. It is built with
`quinn` and is enabled with the Cargo `quic` feature.

```bash
cargo build --features quic
```

The default QUIC bind address is:

```text
0.0.0.0:4433
```

QUIC uses one bidirectional control stream and server-initiated unidirectional
streams for topic data. This gives each topic independent stream flow control
and avoids mixing all topic data into one stream.

## Security Posture

Current QUIC support is designed for trusted local networks. If no certificate
paths are configured, the agent generates a self-signed certificate at startup.
Clients use an insecure verifier so they can connect to that certificate.

Do not treat the current QUIC mode as suitable for untrusted networks. Proper
certificate verification and authentication are future work.

## Choosing A Transport

Use UDS for local development and single-machine workflows. Use QUIC when the
agent runs on a robot and the client runs on a separate developer machine.
