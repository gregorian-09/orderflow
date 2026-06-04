# `of_execution_core` Reference

`of_execution_core` is the canonical execution-domain schema. It intentionally
contains no adapters, threads, sockets, JSON parsing, or broker-specific logic.
The crate exists so every execution layer can agree on the same fixed-size IDs,
typed order requests, execution reports, state transitions, and pre-trade risk
contracts.

## Design Goals

- Keep order data as typed structs on the hot path.
- Use integer-normalized price and quantity fields.
- Keep identifiers fixed-size and ASCII-validated.
- Make state transitions deterministic and testable.
- Keep venue-specific behavior out of the core model.
- Preserve C ABI compatibility through `#[repr(C)]` where applicable.

## Identifier Model

Execution identifiers use `FixedAscii<N>`. This avoids heap allocation for
ordinary IDs, prevents accidental Unicode surprises across FFI boundaries, and
makes identifier capacity explicit.

| Alias | Capacity | Purpose |
| --- | ---: | --- |
| `ClientOrderId` | 40 | Strategy/client-assigned order id |
| `VenueOrderId` | 48 | Venue-assigned order id |
| `ExecutionId` | 48 | Venue fill/report id |
| `AccountId` | 32 | Trading account |
| `RouteId` | 32 | Execution route |
| `StrategyId` | 32 | Strategy attribution |
| `VenueId` | 16 | Venue/exchange id |
| `InstrumentId` | 32 | Venue-native instrument id |
| `ExecutionText` | 128 | Bounded diagnostic text |

### Identifier Rules

- Empty values are allowed where a venue field is not known yet.
- Non-ASCII input is rejected.
- Over-capacity input is rejected.
- Equality and hashing use the validated string content.

This matters because execution code frequently crosses Rust/C/Python/Java
boundaries. `FixedAscii` is the shared guardrail.

## Symbol and Numeric Types

| Type | Meaning |
| --- | --- |
| `ExecutionSymbol { venue, instrument }` | Venue-native execution instrument |
| `OrderQty(i64)` | Integer-normalized quantity |
| `OrderPrice(i64)` | Integer-normalized price |

`OrderQty::new` and `OrderPrice::new` enforce positive values. Raw `OrderQty(0)`
and `OrderPrice(0)` are still used in structs when a field is not applicable,
for example a market order limit price.

## Order Classification

### `OrderSide`

| Variant | Value | Meaning |
| --- | ---: | --- |
| `Buy` | 1 | Buy order |
| `Sell` | 2 | Sell order |

### `OrderType`

| Variant | Value | Meaning |
| --- | ---: | --- |
| `Market` | 1 | Execute at market |
| `Limit` | 2 | Execute at or better than limit |
| `Stop` | 3 | Stop order |
| `StopLimit` | 4 | Stop-limit order |

### `TimeInForce`

| Variant | Value | Meaning |
| --- | ---: | --- |
| `Day` | 1 | Day order |
| `Gtc` | 2 | Good-till-cancelled |
| `Ioc` | 3 | Immediate-or-cancel |
| `Fok` | 4 | Fill-or-kill |
| `Gtd` | 5 | Good-till-date |

## Request Types

### `OrderRequest`

`OrderRequest` is the canonical new-order command.

| Field | Meaning |
| --- | --- |
| `client_order_id` | Unique client id for the order |
| `account_id` | Trading account |
| `route_id` | Execution route |
| `strategy_id` | Strategy attribution |
| `symbol` | Target venue/instrument |
| `side` | Buy or sell |
| `order_type` | Canonical order type |
| `time_in_force` | Canonical TIF |
| `quantity` | Requested quantity |
| `limit_price` | Limit price, or zero when not applicable |
| `stop_price` | Stop price, or zero when not applicable |
| `ts_exchange_ns` | Exchange/session timestamp when known |
| `ts_recv_ns` | Local create/receive timestamp |

`validate()` enforces positive quantity and required price fields for
price-bearing order types.

### `CancelRequest`

Cancel requests carry both a new cancel `client_order_id` and the
`orig_client_order_id` being cancelled. This mirrors FIX-style cancel semantics
and gives adapters enough information for venues that require a new client id
per cancel.

### `AmendRequest`

Amend requests are cancel/replace requests. They carry a new `client_order_id`,
the `orig_client_order_id`, replacement quantity, and replacement limit price.

## Execution Report Types

### `ExecutionType`

`ExecutionType` describes why a report exists: ack, reject, trade, cancel,
replace, expiry, status, restatement, or adapter degradation.

### `OrderStatus`

`OrderStatus` describes the order state after the report is applied. Terminal
states are:

- `Filled`
- `Cancelled`
- `Rejected`
- `Expired`

`OrderStatus::is_terminal()` is used by risk and open-order accounting.

### `ExecutionEvent`

`ExecutionEvent` is the canonical execution report. It carries the current
client order id, optional original client order id, venue order id, execution
id, account/route/symbol, last fill fields, cumulative fields, timestamps,
structured reason, and bounded text.

Two constructors are especially important:

- `ExecutionEvent::accepted(&OrderRequest, VenueOrderId)`
- `ExecutionEvent::rejected(&OrderRequest, RiskRejectReason, ExecutionText)`

Adapters can build their own events directly, but these constructors keep
common local reports consistent.

## Order State Machine

`OrderStateMachine` owns one `OrderState` and applies `ExecutionEvent` values.
It rejects illegal transitions with `ExecutionCoreError::InvalidTransition`.

Typical new order flow:

1. `OrderStateMachine::new(&OrderRequest)` starts at `PendingNew`.
2. `ExecutionType::Ack` moves to `New`.
3. `ExecutionType::Trade` moves to `PartiallyFilled` or `Filled`.
4. `ExecutionType::CancelAck` moves to `Cancelled`.
5. `ExecutionType::ReplaceAck` moves to `Replaced` and updates the accepted
   client order id.

The state machine is intentionally strict. If a report arrives after a terminal
state, only status-style reports are accepted.

## Risk Model

### `RiskLimits`

`RiskLimits` is the basic per-route/account/symbol risk limit struct:

- `kill_switch`
- `max_order_qty`
- `max_order_notional`
- `max_open_orders`
- `max_open_notional`
- `price_band_ticks`

Zero disables numeric checks. `RiskLimits::default()` enables the kill switch,
so test and live route configs must explicitly disable it to allow orders.

### `RiskContext`

The execution engine supplies runtime context:

- open order count
- open notional
- reference price
- duplicate client order id flag
- account/route/symbol enabled flags
- order-type/TIF support flags

The core risk trait does not own runtime state. This keeps custom risk modules
portable.

### `RiskCheck`

`RiskCheck` is the extension point:

- `check_new`
- `check_amend`
- `check_cancel`

`BasicRiskGate` implements this trait with deterministic checks. Higher-level
OMS helpers in `of_execution` add advanced risk utilities while preserving this
contract.

## When To Use This Crate

Use `of_execution_core` when you are:

- writing custom execution adapters,
- implementing strategy-side order creation,
- building custom risk modules,
- testing state transitions,
- integrating through FFI and need canonical field meanings.

Use `of_execution` when you need routing, adapter calls, journaling, concurrent
workers, or OMS utilities.
