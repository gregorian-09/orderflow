# Provider Adapter Authoring

This chapter explains how to add execution adapters for brokers, exchanges,
FIX sessions, REST APIs, WebSocket APIs, or native SDKs.

## Adapter Responsibilities

An execution adapter is responsible for provider I/O and mapping. It is not
responsible for strategy logic or analytics.

An adapter should:

- connect to provider transport,
- expose capabilities,
- translate canonical requests into provider commands,
- translate provider reports into `ExecutionEvent`,
- implement polling/recovery,
- expose health/lifecycle state,
- fail closed when not ready,
- use bounded queues.

## Trait Contract

Implement `ExecutionAdapter`:

```rust
fn connect(&mut self) -> ExecutionResult<()>;
fn submit(&mut self, req: &OrderRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()>;
fn cancel(&mut self, req: &CancelRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()>;
fn amend(&mut self, req: &AmendRequest, out: &mut ExecutionEventBuffer) -> ExecutionResult<()>;
fn poll(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>;
fn recover_open_orders(&mut self, out: &mut ExecutionEventBuffer) -> ExecutionResult<usize>;
fn capabilities(&self) -> ExecutionCapabilities;
fn health(&self) -> ExecutionHealth;
```

## Request Mapping

Map canonical fields explicitly:

| Canonical field | Provider mapping |
| --- | --- |
| `client_order_id` | ClOrdID / clientOrderId / newClientOrderId |
| `account_id` | Account / account key / subaccount |
| `route_id` | Session, venue route, or broker destination |
| `symbol` | Provider-native symbol |
| `side` | Buy/sell encoding |
| `order_type` | Provider order type |
| `time_in_force` | Provider TIF |
| `quantity` | Integer-normalized provider quantity |
| `limit_price` | Integer-normalized provider price |
| `stop_price` | Stop trigger price |

Do not silently coerce unsupported order types. Return a structured error or
let capability checks reject before routing.

## Report Mapping

Every provider report should become `ExecutionEvent`.

Map:

- provider order id to `venue_order_id`,
- provider execution id to `execution_id`,
- last quantity/price to `last_qty` and `last_price`,
- cumulative quantity to `cumulative_qty`,
- leaves quantity to `leaves_qty`,
- average price to `average_price`,
- provider status to `ExecutionType` and `OrderStatus`.

If the provider does not supply a value, use the canonical empty/zero value and
document that behavior.

## FIX Wire Handling

FIX adapters should use `of_fix` for tag-value wire parsing and encoding rather
than implementing a local parser inside each adapter.

Use `of_fix` for:

- borrowed parsing from raw `&[u8]`;
- `BodyLength(9)` and `CheckSum(10)` validation;
- common tag extraction such as `MsgType(35)`, `MsgSeqNum(34)`,
  `PossDupFlag(43)`, `ClOrdID(11)`, `ExecID(17)`, and `OrdStatus(39)`;
- caller-owned encode buffers;
- diagnostic transcript rendering outside hot paths.

Adapter-specific code should own:

- TCP/TLS transport;
- session lifecycle;
- resend/gap-fill policy;
- venue profile validation;
- canonical `ExecutionEvent` mapping;
- certification reports.

## Capabilities

`ExecutionCapabilities` should be honest. If a provider does not support FOK,
set `tif_fok = false`. If a REST adapter cannot preserve native client ids,
set `native_client_order_id = false`.

Capabilities are part of risk and validation. Overstating them creates
unnecessary venue rejects.

## Lifecycle and Health

Use lifecycle states:

- `Disconnected`
- `Connecting`
- `LogonPending`
- `Ready`
- `Recovering`
- `Degraded`
- `Stopped`

`health()` should include:

- connected,
- degraded,
- health sequence,
- last error,
- protocol/session info.

Increase health sequence when the meaningful state changes.

## Recovery

Adapters should implement `recover_open_orders` when the provider supports it.
Recovery reports should use `ExecutionType::Restated`.

Recovery flow:

1. reconnect,
2. authenticate,
3. request open orders,
4. emit restated events,
5. let caller reconcile local and venue state.

## Rate Limits

Provider rate limits should be enforced before sending. Use `OrderThrottle` or
a provider-specific limiter. Separate limits for new/cancel/replace are often
needed.

## Error Handling

Use:

- `ExecutionError::Disconnected` for unavailable transport,
- `ExecutionError::BufferFull` for caller buffer pressure,
- `ExecutionError::Adapter` for provider-specific failures,
- `ExecutionError::Core` for malformed canonical requests,
- `ExecutionError::Journal` only in journal implementations.

Do not hide provider rejects as adapter errors if they are execution reports.
Map them into `ExecutionEvent::Reject`.

## Testing Checklist

For each adapter:

- connects and reports health,
- rejects commands before connect,
- maps ack,
- maps fill,
- maps partial fill,
- maps cancel ack,
- maps cancel reject,
- maps replace ack,
- maps replace reject,
- maps venue reject,
- handles duplicate/out-of-order provider reports,
- recovers open orders,
- respects bounded event buffers,
- reports accurate capabilities.

## Where To Put Adapters

Use `of_execution_adapters` for reusable open-source adapters. Use optional
features for provider SDK dependencies. Keep provider-specific credentials out
of `of_execution_core` and `of_execution`.
