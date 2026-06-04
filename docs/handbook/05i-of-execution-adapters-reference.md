# `of_execution_adapters` Reference

`of_execution_adapters` contains provider-specific execution adapter scaffolds.
It is intentionally separate from `of_execution` so adapter implementations can
remain feature-gated and provider-specific dependencies do not leak into the
core execution crate.

Current adapter scaffold:

- `fix` feature: FIX execution-report mapping and adapter shell.

## Feature Flags

| Feature | Purpose |
| --- | --- |
| `fix` | Enables FIX execution adapter scaffold and report mapper |

The crate should stay dependency-light. New providers should be optional
features unless they are pure standard-library mappings.

## FIX Scaffold

The FIX module is not a full FIX engine. It is a scaffold for building one.
It contains:

- `FixSessionConfig`
- `FixExecutionReport`
- `FixExecType`
- `FixOrdStatus`
- `map_execution_report`
- `FixExecutionAdapter`

### `FixSessionConfig`

| Field | Meaning |
| --- | --- |
| `begin_string` | FIX version such as `FIX.4.4` |
| `sender_comp_id` | SenderCompID |
| `target_comp_id` | TargetCompID |
| `heartbeat_secs` | Heartbeat interval |

All string fields use `FixedAscii` to keep the same identifier discipline as
the execution core.

### `FixExecutionReport`

This is the normalized result of parsing a FIX execution report. It is not the
raw tag map. A real FIX adapter should parse transport bytes into this struct
or an equivalent internal representation, then call `map_execution_report`.

Important fields:

- `exec_type`
- `ord_status`
- `cl_ord_id`
- `orig_cl_ord_id`
- `order_id`
- `exec_id`
- `account_id`
- `route_id`
- `symbol`
- `last_qty`
- `last_price`
- `cumulative_qty`
- `leaves_qty`
- `average_price`
- `ts_exchange_ns`
- `ts_recv_ns`
- `text`

### Mapping Semantics

`map_execution_report` converts FIX-style `ExecType` and `OrdStatus` into the
canonical `ExecutionEvent` model.

Examples:

| FIX concept | Canonical event |
| --- | --- |
| New | `ExecutionType::Ack`, `OrderStatus::New` |
| Trade | `ExecutionType::Trade`, filled or partially filled status |
| Rejected | `ExecutionType::Reject`, `OrderStatus::Rejected` |
| PendingCancel | `ExecutionType::CancelPending` |
| Canceled | `ExecutionType::CancelAck`, `OrderStatus::Cancelled` |
| Replaced | `ExecutionType::ReplaceAck`, `OrderStatus::Replaced` |
| Restated | `ExecutionType::Restated` |

The mapper is deliberately narrow. It does not hide transport behavior,
sequence recovery, session reset, resend, or store management.

## `FixExecutionAdapter`

`FixExecutionAdapter` is a fail-closed shell. Its current behavior is:

- `connect()` returns an adapter error saying transport is not configured.
- `submit`, `cancel`, `amend`, `poll`, and `recover_open_orders` return
  disconnected errors.
- `capabilities()` reports ordinary FIX-style order support.
- `health()` reports degraded/disconnected state.

This is intentional. It gives adapter authors a compilable shape and mapping
contract without pretending a full live FIX session exists.

## Implementing a Real Adapter

A real execution adapter should:

1. own the provider session or transport,
2. validate session lifecycle before accepting commands,
3. map canonical requests into provider messages,
4. map provider reports into `ExecutionEvent`,
5. preserve client-order-id semantics where possible,
6. implement recovery through `recover_open_orders`,
7. expose accurate capabilities,
8. report lifecycle state through `health`,
9. avoid unbounded queues on command/report paths.

For broader guidance, see [Provider Adapter Authoring](./12-provider-adapter-authoring.md).
