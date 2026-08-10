# Binding Documentation

These pages are binding-specific distribution and usage docs.

Latest release: `0.4.0`.

- Python and Java keep the existing analytics/runtime APIs and add separate
  execution classes.
- C ABI symbols for analytics remain stable and add a separate execution symbol
  family.
- The new execution Rust crates publish as `0.1.0`; bindings and native runtime
  publish as `0.4.0`.
- Binding README files include end-to-end examples that combine market data,
  analytics, signal gating, simulated execution, and order reports.
- C, Python, and Java expose a deterministic TWAP parent facade whose owned
  child request must still be submitted through the OMS before release progress
  is committed.
- `bindings/api_manifest.toml` is checked against the C header, exported native
  symbols, Python ctypes registrations, and Java JNA declarations so low-level
  binding plumbing cannot silently drift from the ABI inventory.
- `tools/generate_binding_signatures.py` emits the private Python ctypes
  registrations and Java `OrderflowNative` interface in manifest order from
  exact validated header types; CI checks committed output and unit-tests
  pointer, callback, array, and buffer mappings.
- [Binding API Inventory](./api-inventory.md) is generated from the same
  manifest and lists every exported symbol by family, ownership model, and
  binding exposure level. It also includes a per-symbol C/Python/Java
  compatibility matrix so release reviewers can see binding coverage at a
  glance.

- [Python Binding](./python.md)
- [Java Binding](./java.md)
- [Rust Crates](./rust.md)
- [C ABI](./c.md)

For cross-language concepts and architecture, use the handbook:

- `docs/handbook/README.md`
- `docs/handbook/09-oms-architecture.md`
- `docs/handbook/10-oms-cookbook.md`
