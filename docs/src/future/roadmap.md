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
