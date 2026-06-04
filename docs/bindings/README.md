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

- [Python Binding](./python.md)
- [Java Binding](./java.md)
- [Rust Crates](./rust.md)
- [C ABI](./c.md)

For cross-language concepts and architecture, use the handbook:

- `docs/handbook/README.md`
- `docs/handbook/09-oms-architecture.md`
- `docs/handbook/10-oms-cookbook.md`
