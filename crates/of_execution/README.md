# `of_execution`

`of_execution` provides the routing and adapter layer for Orderflow execution.
It is additive to the existing analytics runtime and does not change market-data
adapter behavior.

The crate is designed around explicit low-latency contracts:

- adapters receive typed request structs and caller-owned event buffers
- event queues are bounded and never silently drop order events
- the simulated adapter is deterministic for integration tests
- journal hooks record command/event outcomes for recovery workflows
- `ConcurrentExecutionEngine` gives concurrent producers a bounded command
  queue while one worker thread owns the deterministic synchronous engine
- OMS helpers cover command correlation, event fanout, lifecycle snapshots,
  durable journaling, reconciliation, safety policies, advanced risk,
  position ledgers, normalization, telemetry, sharding, throttling, replay, and
  provider adapter SDK scaffolding

Real broker and exchange adapters should implement `ExecutionAdapter` and
declare their capabilities and latency class.

The concurrent worker is not a Tokio/async runtime. It uses standard-library
bounded channels and a dedicated owner thread so order state transitions remain
serial, deterministic, and easy to audit.
