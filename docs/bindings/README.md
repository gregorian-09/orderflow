# Binding Documentation

These pages are binding-specific distribution and usage docs.

Latest release: `0.3.0`.

- Python adds type-checker metadata and bundled native-wheel loading support.
- Java keeps the existing JNA API and receives additive runtime metrics through
  the `0.3.0` native library.
- C ABI symbols remain stable; metrics JSON includes additive operational
  fields.
- Current unreleased docs also cover the additive execution/OMS APIs exposed
  through Rust, C, Python, and Java.

- [Python Binding](./python.md)
- [Java Binding](./java.md)
- [Rust Crates](./rust.md)
- [C ABI](./c.md)

For cross-language concepts and architecture, use the handbook:

- `docs/handbook/README.md`
- `docs/handbook/09-oms-architecture.md`
- `docs/handbook/10-oms-cookbook.md`
