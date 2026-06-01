# `of_execution`

`of_execution` provides the routing and adapter layer for Orderflow execution.
It is additive to the existing analytics runtime and does not change market-data
adapter behavior.

The crate is designed around explicit low-latency contracts:

- adapters receive typed request structs and caller-owned event buffers
- event queues are bounded and never silently drop order events
- the simulated adapter is deterministic for integration tests
- journal hooks record command/event outcomes for recovery workflows

Real broker and exchange adapters should implement `ExecutionAdapter` and
declare their capabilities and latency class.

