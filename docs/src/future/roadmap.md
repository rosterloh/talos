# Roadmap

This page tracks likely future work. It is intentionally higher level than an
issue tracker.

## Near Term

- Keep this mdBook as the canonical project documentation.
- Improve examples for agent configuration and common ROS 2 setups.
- Add more integration coverage for UDS and QUIC client behavior.
- Publish the mdBook from `main` with GitHub Pages.

## Protocol And Transport

- Add authenticated QUIC connections for untrusted or shared networks.
- Document and tune QUIC stream limits for large topic sets.
- Improve reconnect behavior and error reporting for remote clients.

## Camera And Media Streaming

A prototype "dumb relay" for camera frames exists: the agent forwards
`sensor_msgs/msg/CompressedImage` payloads verbatim (`{ header, format, data }`)
over the existing topic pipeline, never decoding them. This is enough to observe
a compressed camera feed end to end. Inspired by [Media over QUIC](https://github.com/moq-dev/moq),
the scaling path is:

- Carry frames on a dedicated binary frame type instead of wrapping bytes in a
  `DynValue` tree.
- Open a new QUIC stream per keyframe group (GOP) so a slow client drops stale
  frames instead of head-of-line blocking on one long-lived reliable stream.
- Keep the agent codec-free (forward `CompressedImage` only); never add an
  encoder. Decoding stays a client concern.
- Consider a non-terminal viewer (browser over WebTransport, or a native
  window) — the ratatui TUI cannot render real video beyond low-fps
  sixel/kitty previews.
- Reject raw `sensor_msgs/msg/Image`: at full frame rate it exceeds the frame
  cap and saturates the reliable pipeline.

## ROS 2 Coverage

- Add support for more common ROS 2 message types.
- Explore generic message conversion instead of compiled-in conversions only.
- Consider service and action proxying once topic observation and control are
stable.

## User Experience

- Improve TUI ergonomics for filtering, selection, and high-rate streams.
- Add examples for scripting with the CLI.
- Make joint control behavior clearer and safer around limits and command
publishing.

## Documentation

- Link important Rust API items from this book where they clarify concepts.
- Optionally publish Rustdoc separately from the mdBook.
- Keep design history concise and focused on accepted decisions.
