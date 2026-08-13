# Subsystem Audit Template

Each crate and binding surface is audited using the same sequence. This keeps
the knowledge system consistent as Orderflow grows.

## Identity and Scope

- Package/module name and version.
- Purpose and non-goals.
- Dependency and feature graph.
- Supported platforms and language bindings.
- Source-of-truth files.

## Public Surface

For every public module, struct, field, enum, variant, trait, method, function,
type alias, constant, static, error, and feature:

- exact name and qualified path;
- exact type and representation/layout;
- default, range, sentinel, and zero-value behavior;
- ownership, borrowing, allocation, and lifetime behavior;
- error behavior and invalid-input handling;
- thread-safety, reentrancy, and callback behavior;
- determinism and ordering guarantees;
- persistence/serialization behavior;
- introduction version and compatibility status.

## Behavioral Model

- State machine and legal transitions.
- Input validation order.
- Event ordering and duplicate policy.
- Reset, close, restart, and recovery behavior.
- Backpressure and resource limits.
- Observability and diagnostic fields.

## Performance Model

- Hot-path and control-plane boundaries.
- Allocation behavior after warm-up.
- Blocking operations.
- Queue bounds and memory growth.
- Timestamp and latency measurement points.
- Feature costs and opt-in compilation.

## Evidence

- Unit tests for invariants.
- Integration tests for cross-crate behavior.
- Binding smoke tests.
- Runnable examples.
- Serialization fixtures and migration tests.
- Documentation links and generated-reference links.

## Completion States

`inventory` means the source surface is listed. `scaffold` means a destination
page exists. `documented` means the public and behavioral contract is written.
`verified` requires automated checks and executable evidence.
