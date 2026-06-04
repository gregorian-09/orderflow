# `of_execution_core`

[![Crates.io](https://img.shields.io/crates/v/of_execution_core.svg)](https://crates.io/crates/of_execution_core)
[![Docs.rs](https://docs.rs/of_execution_core/badge.svg)](https://docs.rs/of_execution_core)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

`of_execution_core` is the canonical execution-domain model for Orderflow.
It contains the low-level types used by the order-management and execution
layers: fixed-size identifiers, typed order requests, execution reports,
order-state transitions, and pre-trade risk contracts.

This crate is intentionally small. It has no broker SDKs, no sockets, no worker
threads, no JSON payloads, and no dependency on the market-data analytics
runtime. Higher-level crates build on it:

- `of_execution` adds routes, adapters, journals, simulated execution,
  concurrency, and OMS helpers.
- `of_execution_adapters` adds optional provider adapter scaffolds.
- `of_ffi_c`, Python, and Java expose execution APIs through additive handles
  and classes.

## First Release: 0.1.0

`of_execution_core` starts at `0.1.0` even though the broader Orderflow release
line is `0.4.0`. This crate is a new public execution-domain surface. It is
kept independent from the mature analytics crates so its identifiers, state
machine, and risk contracts can stabilize honestly without implying the same
API age as `of_core`.

Compatibility expectations:

- all public APIs are intended to be additive where possible;
- provider adapters should pin compatible `0.1.x` versions while the execution
  trait family matures;
- analytics crates do not depend on this crate;
- bindings expose these concepts through additive execution handles/classes,
  not by changing existing analytics types.

## Design Goals

- Keep hot-path order data as typed structs, not dynamic maps.
- Use fixed-size ASCII identifiers instead of heap-owned strings.
- Use integer-normalized prices and quantities.
- Preserve deterministic, auditable state transitions.
- Keep provider-specific behavior outside the core schema.
- Make pre-trade risk decisions structured and testable.
- Keep the model suitable for Rust, C ABI, Python, and Java integration.

## Public API Inventory

Constants:

- [`EXECUTION_TEXT_CAP`]

Identifier types:

- [`FixedAscii<N>`]
- [`ClientOrderId`]
- [`VenueOrderId`]
- [`ExecutionId`]
- [`AccountId`]
- [`RouteId`]
- [`StrategyId`]
- [`VenueId`]
- [`InstrumentId`]
- [`ExecutionText`]

Errors and domain types:

- [`ExecutionCoreError`]
- [`ExecutionSymbol`]
- [`OrderQty`]
- [`OrderPrice`]
- [`OrderSide`]
- [`OrderType`]
- [`TimeInForce`]
- [`OrderStatus`]
- [`ExecutionType`]

Request and report types:

- [`OrderRequest`]
- [`CancelRequest`]
- [`AmendRequest`]
- [`ExecutionEvent`]

State-machine types:

- [`OrderState`]
- [`OrderStateMachine`]

Risk types:

- [`RiskRejectReason`]
- [`RiskDecision`]
- [`RiskLimits`]
- [`RiskContext`]
- [`RiskCheck`]
- [`BasicRiskGate`]

## Identifier Model

Execution identifiers use [`FixedAscii<N>`]. A fixed ASCII value stores a length
and an inline byte array. This avoids heap allocation in ordinary command and
report structs, keeps FFI boundaries predictable, and prevents accidental
Unicode normalization differences between host languages.

| Alias | Capacity | Purpose |
| --- | ---: | --- |
| [`ClientOrderId`] | 40 | Strategy/client-assigned order id |
| [`VenueOrderId`] | 48 | Venue-assigned order id |
| [`ExecutionId`] | 48 | Venue fill/report id |
| [`AccountId`] | 32 | Trading account |
| [`RouteId`] | 32 | Execution route |
| [`StrategyId`] | 32 | Strategy attribution |
| [`VenueId`] | 16 | Venue/exchange id |
| [`InstrumentId`] | 32 | Venue-native instrument id |
| [`ExecutionText`] | 128 | Bounded diagnostic text |

Identifier rules:

- Empty identifiers are allowed where a venue value is not known yet.
- Non-ASCII input returns [`ExecutionCoreError::NonAsciiIdentifier`].
- Over-capacity input returns [`ExecutionCoreError::IdentifierTooLong`].
- Equality, hashing, `Debug`, and `Display` use the validated string content.

Example:

```rust
use of_execution_core::{ClientOrderId, ExecutionCoreError};

let id = ClientOrderId::new("STRAT-0001")?;
assert_eq!(id.as_str(), "STRAT-0001");
assert_eq!(id.capacity(), 40);
assert!(!id.is_empty());

let too_long = "X".repeat(41);
assert!(matches!(
    ClientOrderId::new(&too_long),
    Err(ExecutionCoreError::IdentifierTooLong { .. })
));
# Ok::<(), ExecutionCoreError>(())
```

## Symbols, Quantity, And Price

[`ExecutionSymbol`] is the venue-native execution symbol:

- `venue`: venue or exchange identifier
- `instrument`: venue-native instrument id

[`OrderQty`] and [`OrderPrice`] are integer-normalized wrappers. Use symbol
metadata outside this crate to convert user-facing decimals into the integer
representation expected by a provider.

`OrderQty::new` rejects zero and negative values. `OrderPrice::new` rejects
zero and negative values. Some request fields still use `OrderPrice(0)` as a
sentinel when a price is not applicable, for example a market order limit
price.

```rust
use of_execution_core::{ExecutionSymbol, OrderPrice, OrderQty};

let symbol = ExecutionSymbol::new("CME", "ESM6")?;
let qty = OrderQty::new(10)?;
let price = OrderPrice::new(505_000)?;

assert_eq!(symbol.instrument.as_str(), "ESM6");
assert_eq!(qty.0, 10);
assert_eq!(price.0, 505_000);
# Ok::<(), of_execution_core::ExecutionCoreError>(())
```

## Order Classification

[`OrderSide`] variants:

- `Buy`
- `Sell`

[`OrderType`] variants:

- `Market`
- `Limit`
- `Stop`
- `StopLimit`

[`TimeInForce`] variants:

- `Day`
- `Gtc`
- `Ioc`
- `Fok`
- `Gtd`

These are canonical values. Provider adapters can translate them to native
protocol values or reject unsupported combinations through capabilities/risk.

## Request Types

### `OrderRequest`

[`OrderRequest`] is the canonical new-order command.

Fields:

- `client_order_id`: client-assigned id for the new order
- `account_id`: trading account
- `route_id`: execution route
- `strategy_id`: strategy attribution
- `symbol`: target venue/instrument
- `side`: buy/sell
- `order_type`: canonical type
- `time_in_force`: canonical TIF
- `quantity`: requested quantity
- `limit_price`: limit price, or zero when not applicable
- `stop_price`: stop trigger price, or zero when not applicable
- `ts_exchange_ns`: exchange/session timestamp when known
- `ts_recv_ns`: local creation/receive timestamp

`OrderRequest::validate` checks:

- quantity must be positive
- limit orders require positive `limit_price`
- stop orders require positive `stop_price`
- stop-limit orders require both positive `limit_price` and `stop_price`

### `CancelRequest`

[`CancelRequest`] carries:

- `client_order_id`: client id for the cancel request itself
- `orig_client_order_id`: client id of the order being cancelled
- `account_id`
- `route_id`
- `symbol`
- `ts_exchange_ns`
- `ts_recv_ns`

This mirrors FIX-style cancel semantics where the cancel command may need a new
client id separate from the original order id.

### `AmendRequest`

[`AmendRequest`] is the canonical cancel/replace command. It carries a new
client id, the original client id, replacement quantity, replacement limit
price, and timestamps. The higher-level engine checks that the original order
is locally known before routing the request.

## Execution Reports

[`ExecutionType`] explains why an execution event exists:

- `Ack`
- `Reject`
- `Trade`
- `CancelPending`
- `CancelAck`
- `CancelReject`
- `ReplacePending`
- `ReplaceAck`
- `ReplaceReject`
- `Expired`
- `Status`
- `Restated`
- `Degraded`

[`OrderStatus`] explains the state after the event is applied. Terminal states
are detected with `OrderStatus::is_terminal()`.

[`ExecutionEvent`] is the canonical report shape. It includes:

- current and original client order ids
- venue order id
- execution id
- account, route, and symbol
- execution type and resulting order status
- last fill quantity/price
- cumulative quantity
- leaves quantity
- average price
- exchange and receive timestamps
- structured rejection reason
- bounded diagnostic text

Constructors:

- `ExecutionEvent::accepted(&OrderRequest, VenueOrderId)`
- `ExecutionEvent::rejected(&OrderRequest, RiskRejectReason, ExecutionText)`

Provider adapters can construct events directly, but these helpers keep common
local ack/reject reports consistent.

## Order State Machine

[`OrderStateMachine`] owns one [`OrderState`] and applies [`ExecutionEvent`]
values. Illegal transitions return [`ExecutionCoreError::InvalidTransition`].

Typical lifecycle:

1. `OrderStateMachine::new(&request)` starts at `PendingNew`.
2. `ExecutionType::Ack` moves to `New`.
3. `ExecutionType::Trade` moves to `PartiallyFilled` or `Filled`.
4. `ExecutionType::CancelAck` moves to `Cancelled`.
5. `ExecutionType::ReplaceAck` moves to `Replaced` and updates the active
   client order id.
6. `ExecutionType::Restated` updates local state from recovery reports.

The state machine is deliberately strict. It accepts status-style reports after
terminal states, but rejects ordinary lifecycle transitions that would mutate a
terminal order.

```rust
use of_execution_core::{
    AccountId, ClientOrderId, ExecutionEvent, ExecutionSymbol, OrderPrice,
    OrderQty, OrderRequest, OrderSide, OrderStateMachine, OrderType, RouteId,
    StrategyId, TimeInForce, VenueOrderId,
};

let req = OrderRequest {
    client_order_id: ClientOrderId::new("C1")?,
    account_id: AccountId::new("ACC")?,
    route_id: RouteId::new("SIM")?,
    strategy_id: StrategyId::new("STRAT")?,
    symbol: ExecutionSymbol::new("SIM", "ES")?,
    side: OrderSide::Buy,
    order_type: OrderType::Limit,
    time_in_force: TimeInForce::Day,
    quantity: OrderQty::new(1)?,
    limit_price: OrderPrice::new(5000)?,
    stop_price: OrderPrice(0),
    ts_exchange_ns: 0,
    ts_recv_ns: 1,
};

let mut state = OrderStateMachine::new(&req);
let ack = ExecutionEvent::accepted(&req, VenueOrderId::new("V1")?);
state.apply(&ack)?;
assert_eq!(state.state().client_order_id, req.client_order_id);
# Ok::<(), of_execution_core::ExecutionCoreError>(())
```

## Risk Model

[`RiskLimits`] is the basic per-route/account/symbol risk configuration:

- `kill_switch`
- `max_order_qty`
- `max_order_notional`
- `max_open_orders`
- `max_open_notional`
- `price_band_ticks`

Zero disables numeric checks. `RiskLimits::default()` enables the kill switch,
so callers must explicitly disable it for test or live routes that should
accept orders.

[`RiskContext`] is supplied by higher-level engines. It includes runtime facts:

- open order count
- open notional
- reference price
- duplicate client order id flag
- whether the account, route, and symbol are enabled
- whether the order type and TIF are supported

[`RiskCheck`] is the extension trait:

- `check_new`
- `check_amend`
- `check_cancel`

[`BasicRiskGate`] implements deterministic checks for the basic limit set.

```rust
use of_execution_core::{BasicRiskGate, RiskCheck, RiskLimits};

let gate = BasicRiskGate::new(RiskLimits {
    kill_switch: false,
    max_order_qty: 100,
    max_order_notional: 1_000_000,
    max_open_orders: 5,
    max_open_notional: 5_000_000,
    price_band_ticks: 0,
});

// Higher-level engines build RiskContext and call the gate before routing.
let _ = gate;
```

## Low-Latency Notes

- Requests and reports are fixed-layout structs.
- Identifiers are inline fixed-size ASCII fields.
- Numeric fields are integers.
- Validation is explicit and local.
- The crate does not allocate for ordinary order/request/event structs.
- Provider strings and protocol-specific payloads should be translated at the
  adapter boundary before entering this model.

## What This Crate Does Not Do

This crate does not:

- connect to brokers or exchanges
- own order routes
- run worker threads
- expose a C ABI directly
- persist journals
- reconcile venue open orders
- enforce provider rate limits
- parse FIX/REST/WebSocket messages

Use `of_execution` for routing, adapter calls, simulated execution,
journaling, recovery, and OMS helpers.

## Documentation

Additional project documentation:

- `docs/handbook/05g-of-execution-core-reference.md`
- `docs/handbook/09-oms-architecture.md`
- `docs/handbook/11-low-latency-design.md`
