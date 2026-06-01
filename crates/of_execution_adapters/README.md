# `of_execution_adapters`

`of_execution_adapters` contains optional broker and venue execution adapter
scaffolds for Orderflow.

The first scaffold is FIX-oriented. It provides:

- native FIX latency classification
- typed FIX execution-report mapping into canonical `ExecutionEvent` values
- an adapter shell that fails closed until a real transport is wired

This crate deliberately keeps provider-specific transports feature-gated and
outside the analytics runtime.

