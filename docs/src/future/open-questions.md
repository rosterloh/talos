# Open Questions

## Security

What is the right authentication model for remote access? Current QUIC support
is suitable for trusted local networks only.

## Dynamic Message Support

Should Talos continue adding compiled-in conversions for common message types,
or should it support generic ROS 2 message introspection?

Compiled-in conversions are simple and predictable. Generic conversion would
cover more systems but adds complexity around type discovery, field traversal,
and compatibility.

## Services And Actions

Topic observation is the first priority. Service and action proxying could make
Talos more capable, but they would expand the protocol and UI model
significantly.

## Documentation Versioning

The current plan is `main`/latest documentation only. If Talos starts shipping
stable releases with incompatible behavior, versioned docs may become useful.

## Rustdoc Publishing

Rustdoc should remain separate from this book. The open question is whether CI
should also publish Rustdoc under a stable path, such as `/api/`, next to the
mdBook output.
