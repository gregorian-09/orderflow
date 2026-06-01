# `of_execution_core`

`of_execution_core` contains the additive execution-domain model for
Orderflow. It is intentionally independent from market-data adapters and the
analytics runtime.

The crate is designed for low-latency execution paths:

- fixed-size ASCII identifiers instead of heap-owned strings
- typed request and event structs instead of JSON payloads
- explicit order-state transitions based on FIX-style execution semantics
- pre-trade risk decisions with structured rejection reasons

This crate does not connect to brokers or exchanges. Adapter and routing logic
belongs in higher-level execution crates.

