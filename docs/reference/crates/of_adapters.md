# `of_adapters` Reference

> Generated from `crates/of_adapters/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.5.0`<br>
**Description:** Provider adapters and market-data abstraction for the Orderflow engine<br>
**Source:** [`crates/of_adapters/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_adapters/src)<br>
**Generated Rustdoc:** [open `of_adapters` Rustdoc](https://docs.rs/of_adapters/0.5.0/of_adapters/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- `default`: empty feature
- `rithmic`: empty feature
- `cqg`: empty feature
- `cqg_proto`: `cqg`
- `binance`: empty feature

## Local Dependencies

- [`of_core`](./of_core.md)

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `struct` | `BinanceConfig` | Resolved Binance adapter configuration | [`crates/of_adapters/src/binance.rs:64`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L64) | `present` |
| `fn` | `from_adapter_config` | Builds Binance config from generic adapter config with default mock endpoint | [`crates/of_adapters/src/binance.rs:70`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L70) | `present` |
| `struct` | `BinanceAdapter` | Binance websocket adapter with mock/live transport support | [`crates/of_adapters/src/binance.rs:237`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L237) | `present` |
| `fn` | `from_config` | Creates a Binance adapter from generic adapter configuration | [`crates/of_adapters/src/binance.rs:277`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L277) | `present` |
| `fn` | `with_max_queue_depth` | Returns a copy of this adapter with a maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:329`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L329) | `present` |
| `fn` | `set_max_queue_depth` | Sets the maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:338`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L338) | `present` |
| `fn` | `max_queue_depth` | Returns the configured maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:346`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L346) | `present` |
| `fn` | `with_raw_capture_capacity` | Returns a copy of this adapter with raw inbound message capture enabled | [`crates/of_adapters/src/binance.rs:355`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L355) | `present` |
| `fn` | `set_raw_capture_capacity` | Sets the raw inbound message capture capacity | [`crates/of_adapters/src/binance.rs:364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L364) | `present` |
| `fn` | `raw_capture_capacity` | Returns the configured raw inbound message capture capacity | [`crates/of_adapters/src/binance.rs:379`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L379) | `present` |
| `fn` | `raw_capture_len` | Returns the number of raw inbound messages currently buffered | [`crates/of_adapters/src/binance.rs:384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L384) | `present` |
| `fn` | `raw_capture_dropped` | Returns the number of raw inbound messages dropped from the capture ring | [`crates/of_adapters/src/binance.rs:389`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L389) | `present` |
| `fn` | `drain_raw_messages` | Drains captured raw inbound messages into `out` | [`crates/of_adapters/src/binance.rs:397`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L397) | `present` |
| `fn` | `replay_raw_messages` | Replays raw Binance JSON messages into normalized events | [`crates/of_adapters/src/binance.rs:410`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L410) | `present` |
| `struct` | `BookSequencer` | Tracks the last accepted market-data sequence for each CQG contract | [`crates/of_adapters/src/cqg/book.rs:5`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L5) | `present` |
| `fn` | `apply_sequence` | Classifies a sequence and records it when it is usable for progression | [`crates/of_adapters/src/cqg/book.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L11) | `present` |
| `enum` | `SequenceStatus` | Result of applying a market-data sequence to a contract stream | [`crates/of_adapters/src/cqg/book.rs:32`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L32) | `present` |
| `struct` | `CqgConfig` | Resolved CQG adapter runtime configuration | [`crates/of_adapters/src/cqg/config.rs:5`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L5) | `present` |
| `fn` | `from_adapter_config` | Builds CQG config from generic adapter configuration plus environment vars | [`crates/of_adapters/src/cqg/config.rs:30`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L30) | `present` |
| `fn` | `validate_runtime` | Validates runtime invariants for reconnect and heartbeat policies | [`crates/of_adapters/src/cqg/config.rs:70`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L70) | `present` |
| `fn` | `map_inbound_to_raw` | Maps a decoded CQG event into the normalized raw adapter event model | [`crates/of_adapters/src/cqg/mapper.rs:7`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/mapper.rs#L7) | `present` |
| `struct` | `CqgMetrics` | Counters and timing observations collected by the CQG adapter | [`crates/of_adapters/src/cqg/metrics.rs:3`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L3) | `present` |
| `struct` | `CqgAdapter` | CQG adapter implementation with session/reconnect/heartbeat supervision | [`crates/of_adapters/src/cqg/mod.rs:29`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/mod.rs#L29) | `present` |
| `fn` | `from_config` | Creates a CQG adapter and validates runtime-safe configuration | [`crates/of_adapters/src/cqg/mod.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/mod.rs#L47) | `present` |
| `enum` | `CqgOutbound` | Outbound messages supported by the CQG transport protocol | [`crates/of_adapters/src/cqg/proto.rs:3`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L3) | `present` |
| `enum` | `CqgInbound` | Inbound messages supported by the CQG transport protocol | [`crates/of_adapters/src/cqg/proto.rs:20`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L20) | `present` |
| `fn` | `encode_outbound` | Encodes an outbound message using the CQG protobuf-compatible frame format | [`crates/of_adapters/src/cqg/proto.rs:79`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L79) | `present` |
| `fn` | `encode_inbound_for_test` | Encodes an inbound message for deterministic transport and codec tests | [`crates/of_adapters/src/cqg/proto.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L105) | `present` |
| `fn` | `decode_inbound` | Decodes one CQG protobuf-compatible frame into a normalized inbound message | [`crates/of_adapters/src/cqg/proto.rs:180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L180) | `present` |
| `fn` | `is_ping_outbound_frame` | Returns whether a frame is a CQG ping message | [`crates/of_adapters/src/cqg/proto.rs:266`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L266) | `present` |
| `fn` | `encode_outbound` | Encodes an outbound CQG message using the enabled protobuf codec | [`crates/of_adapters/src/cqg/proto.rs:455`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L455) | `present` |
| `fn` | `pb_schema_version` | Returns the CQG wire schema version implemented by this build | [`crates/of_adapters/src/cqg/proto.rs:461`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L461) | `present` |
| `fn` | `pb_schema_version` | Returns zero when the optional CQG protobuf codec is disabled | [`crates/of_adapters/src/cqg/proto.rs:467`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L467) | `present` |
| `fn` | `encode_outbound` | Encodes an outbound CQG message with the fallback text codec | [`crates/of_adapters/src/cqg/proto.rs:473`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L473) | `present` |
| `fn` | `encode_inbound_for_test` | Encodes an inbound CQG message for deterministic tests | [`crates/of_adapters/src/cqg/proto.rs:492`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L492) | `present` |
| `fn` | `encode_inbound_for_test` | Encodes an inbound CQG message with the fallback text codec | [`crates/of_adapters/src/cqg/proto.rs:498`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L498) | `present` |
| `fn` | `decode_inbound` | Decodes one inbound CQG frame with the enabled protobuf codec | [`crates/of_adapters/src/cqg/proto.rs:560`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L560) | `present` |
| `fn` | `decode_inbound` | Decodes one inbound CQG frame with the fallback text codec | [`crates/of_adapters/src/cqg/proto.rs:566`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L566) | `present` |
| `fn` | `is_ping_outbound_frame` | Returns whether a frame is a CQG ping message | [`crates/of_adapters/src/cqg/proto.rs:645`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L645) | `present` |
| `fn` | `is_ping_outbound_frame` | Returns whether a fallback frame is a CQG ping message | [`crates/of_adapters/src/cqg/proto.rs:651`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L651) | `present` |
| `fn` | `wire_mode` | Returns the active CQG wire mode name | [`crates/of_adapters/src/cqg/proto.rs:657`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L657) | `present` |
| `fn` | `wire_mode` | Returns the fallback CQG wire mode name | [`crates/of_adapters/src/cqg/proto.rs:663`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L663) | `present` |
| `enum` | `CqgSessionState` | Lifecycle state of a CQG session | [`crates/of_adapters/src/cqg/session.rs:7`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L7) | `present` |
| `struct` | `CqgSession` | Correlates CQG symbol and subscription requests with their acknowledgements | [`crates/of_adapters/src/cqg/session.rs:20`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L20) | `present` |
| `fn` | `new` | Creates a disconnected CQG session with request identifiers starting at one | [`crates/of_adapters/src/cqg/session.rs:31`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L31) | `present` |
| `fn` | `state` | Returns the current session lifecycle state | [`crates/of_adapters/src/cqg/session.rs:43`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L43) | `present` |
| `fn` | `set_state` | Sets the current session lifecycle state | [`crates/of_adapters/src/cqg/session.rs:48`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L48) | `present` |
| `fn` | `next_request_id` | Allocates the next non-zero request identifier | [`crates/of_adapters/src/cqg/session.rs:53`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L53) | `present` |
| `fn` | `queue_symbol_resolution` | Queues symbol resolution and returns its request identifier | [`crates/of_adapters/src/cqg/session.rs:60`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L60) | `present` |
| `fn` | `on_symbol_resolved` | Completes symbol resolution and returns the symbol and requested depth | [`crates/of_adapters/src/cqg/session.rs:69`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L69) | `present` |
| `fn` | `queue_subscription_ack` | Records the expected subscription acknowledgement for a request | [`crates/of_adapters/src/cqg/session.rs:81`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L81) | `present` |
| `fn` | `on_subscription_ack` | Resolves a subscription acknowledgement request, if it is pending | [`crates/of_adapters/src/cqg/session.rs:87`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L87) | `present` |
| `fn` | `has_pending_work` | Returns whether symbol or subscription requests are awaiting responses | [`crates/of_adapters/src/cqg/session.rs:92`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L92) | `present` |
| `fn` | `clear_transient` | Removes requests that cannot survive a reconnect | [`crates/of_adapters/src/cqg/session.rs:97`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L97) | `present` |
| `fn` | `upsert_requested_depth` | Records the desired market-data depth for a symbol | [`crates/of_adapters/src/cqg/session.rs:103`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L103) | `present` |
| `fn` | `remove_symbol` | Removes a symbol and all pending requests associated with it | [`crates/of_adapters/src/cqg/session.rs:108`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L108) | `present` |
| `trait` | `CqgTransport` | Transport boundary used by the CQG session and adapter | [`crates/of_adapters/src/cqg/transport.rs:12`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/transport.rs#L12) | `present` |
| `struct` | `MockTransport` | In-memory CQG transport used by tests and deterministic simulations | [`crates/of_adapters/src/cqg/transport.rs:23`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/transport.rs#L23) | `present` |
| `struct` | `WsProtobufTransport` | TCP-backed WebSocket/protobuf transport for CQG sessions | [`crates/of_adapters/src/cqg/transport.rs:59`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/transport.rs#L59) | `present` |
| `fn` | `new` | Creates a transport for the supplied CQG endpoint | [`crates/of_adapters/src/cqg/transport.rs:68`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/transport.rs#L68) | `present` |
| `struct` | `SubscribeReq` | Subscription request forwarded to adapters | [`crates/of_adapters/src/lib.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L13) | `present` |
| `struct` | `AdapterHealth` | Adapter connection and quality health snapshot | [`crates/of_adapters/src/lib.rs:22`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L22) | `present` |
| `enum` | `AdapterRuntimeMode` | Transport mode used by an active adapter instance | [`crates/of_adapters/src/lib.rs:36`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L36) | `present` |
| `fn` | `id` | Returns the stable lowercase identifier used in status payloads | [`crates/of_adapters/src/lib.rs:52`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L52) | `present` |
| `enum` | `AdapterConnectionState` | Provider connection state reported by an active adapter instance | [`crates/of_adapters/src/lib.rs:66`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L66) | `present` |
| `fn` | `id` | Returns the stable lowercase identifier used in status payloads | [`crates/of_adapters/src/lib.rs:86`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L86) | `present` |
| `struct` | `AdapterOperationalStatus` | Typed operational status for a market-data adapter | [`crates/of_adapters/src/lib.rs:108`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L108) | `present` |
| `fn` | `new` | Creates an operational snapshot with the supplied mode and state | [`crates/of_adapters/src/lib.rs:147`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L147) | `present` |
| `fn` | `with_mode` | Sets the transport mode | [`crates/of_adapters/src/lib.rs:156`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L156) | `present` |
| `fn` | `with_connection_state` | Sets the provider connection state | [`crates/of_adapters/src/lib.rs:162`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L162) | `present` |
| `fn` | `with_endpoint` | Redacts and sets a configured endpoint | [`crates/of_adapters/src/lib.rs:168`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L168) | `present` |
| `fn` | `with_app_name` | Sets a non-secret provider application name | [`crates/of_adapters/src/lib.rs:174`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L174) | `present` |
| `fn` | `with_reconnect_attempt` | Sets the current reconnect attempt number | [`crates/of_adapters/src/lib.rs:180`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L180) | `present` |
| `fn` | `with_subscribed_symbols` | Sets and deterministically orders active subscriptions | [`crates/of_adapters/src/lib.rs:186`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L186) | `present` |
| `fn` | `with_queue` | Sets provider-event queue utilization | [`crates/of_adapters/src/lib.rs:200`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L200) | `present` |
| `fn` | `with_loss_counters` | Sets drop and sequence-gap counters | [`crates/of_adapters/src/lib.rs:207`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L207) | `present` |
| `fn` | `with_stale` | Sets the adapter-specific stale-feed state | [`crates/of_adapters/src/lib.rs:214`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L214) | `present` |
| `fn` | `with_raw_capture` | Sets bounded raw-message capture utilization | [`crates/of_adapters/src/lib.rs:220`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L220) | `present` |
| `fn` | `with_activity_ages` | Sets provider-message and normalized-event ages | [`crates/of_adapters/src/lib.rs:228`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L228) | `present` |
| `fn` | `redact_adapter_endpoint` | Returns a diagnostics-safe endpoint containing only URI scheme and authority | [`crates/of_adapters/src/lib.rs:243`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L243) | `present` |
| `enum` | `RawEvent` | Raw adapter event stream | [`crates/of_adapters/src/lib.rs:274`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L274) | `present` |
| `enum` | `AdapterError` | Adapter-level error variants | [`crates/of_adapters/src/lib.rs:283`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L283) | `present` |
| `type` | `AdapterResult` | Result type alias used by adapter interfaces | [`crates/of_adapters/src/lib.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L308) | `present` |
| `trait` | `MarketDataAdapter` | Common market-data adapter interface used by runtime | [`crates/of_adapters/src/lib.rs:311`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L311) | `present` |
| `enum` | `ProviderKind` | Provider selection used by adapter factory configuration | [`crates/of_adapters/src/lib.rs:359`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L359) | `present` |
| `fn` | `id` | Returns the stable lowercase provider id used in diagnostics | [`crates/of_adapters/src/lib.rs:372`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L372) | `present` |
| `enum` | `AdapterQualityLevel` | Adapter maturity level advertised by the discovery registry | [`crates/of_adapters/src/lib.rs:385`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L385) | `present` |
| `fn` | `id` | Returns the stable lowercase quality id used in diagnostics | [`crates/of_adapters/src/lib.rs:408`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L408) | `present` |
| `fn` | `rank` | Returns the conservative ordering used by conformance reports | [`crates/of_adapters/src/lib.rs:423`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L423) | `present` |
| `fn` | `meets` | Returns true when this level is at least as mature as `target` | [`crates/of_adapters/src/lib.rs:438`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L438) | `present` |
| `struct` | `AdapterDescriptor` | Static capability description for one market-data adapter | [`crates/of_adapters/src/lib.rs:446`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L446) | `present` |
| `enum` | `AdapterConformanceRequirement` | Adapter conformance requirement checked for a target quality level | [`crates/of_adapters/src/lib.rs:496`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L496) | `present` |
| `fn` | `id` | Returns the stable lowercase requirement id used in reports | [`crates/of_adapters/src/lib.rs:531`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L531) | `present` |
| `struct` | `AdapterConformanceFailure` | One failed adapter conformance requirement | [`crates/of_adapters/src/lib.rs:554`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L554) | `present` |
| `struct` | `AdapterConformanceReport` | Adapter conformance report for one descriptor and target quality level | [`crates/of_adapters/src/lib.rs:563`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L563) | `present` |
| `fn` | `passed` | Returns true when every checked requirement passed | [`crates/of_adapters/src/lib.rs:580`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L580) | `present` |
| `fn` | `adapter_quality_requirements` | Returns the conformance requirements for a target adapter quality level | [`crates/of_adapters/src/lib.rs:652`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L652) | `present` |
| `fn` | `adapter_descriptors` | Returns static descriptors for all known adapter providers | [`crates/of_adapters/src/lib.rs:770`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L770) | `present` |
| `fn` | `compiled_adapter_descriptors` | Returns descriptors for providers compiled into the current binary | [`crates/of_adapters/src/lib.rs:775`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L775) | `present` |
| `fn` | `describe_adapter` | Returns the descriptor for `provider` | [`crates/of_adapters/src/lib.rs:784`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L784) | `present` |
| `fn` | `adapter_feature_enabled` | Returns true when the current binary can construct `provider` | [`crates/of_adapters/src/lib.rs:793`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L793) | `present` |
| `fn` | `evaluate_adapter_conformance` | Evaluates whether a descriptor satisfies a target adapter quality level | [`crates/of_adapters/src/lib.rs:801`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L801) | `present` |
| `fn` | `adapter_conformance_report` | Evaluates a known provider against a target adapter quality level | [`crates/of_adapters/src/lib.rs:835`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L835) | `present` |
| `struct` | `AdapterConfig` | Generic adapter factory configuration | [`crates/of_adapters/src/lib.rs:902`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L902) | `present` |
| `struct` | `CredentialsRef` | Credential environment-variable references for adapter auth bootstrap | [`crates/of_adapters/src/lib.rs:926`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L926) | `present` |
| `fn` | `create_adapter` | Creates a provider adapter from configuration | [`crates/of_adapters/src/lib.rs:1047`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1047) | `present` |
| `struct` | `MockAdapter` | Deterministic in-memory adapter for tests, demos, and replay harnesses | [`crates/of_adapters/src/lib.rs:1106`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1106) | `present` |
| `fn` | `push_event` | Pushes an event into mock queue, drained by `poll` | [`crates/of_adapters/src/lib.rs:1116`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1116) | `present` |
| `mod` | `rithmic` | Rithmic adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1175`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1175) | `present` |
| `mod` | `cqg` | CQG adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1179`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1179) | `present` |
| `mod` | `binance` | Binance adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1183`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1183) | `present` |
| `struct` | `RithmicConfig` | Resolved runtime configuration for the feature-gated Rithmic adapter | [`crates/of_adapters/src/rithmic.rs:19`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L19) | `present` |
| `fn` | `from_adapter_config` | Builds a validated Rithmic config from generic adapter config input | [`crates/of_adapters/src/rithmic.rs:28`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L28) | `present` |
| `struct` | `RithmicAdapter` | Rithmic adapter implementing the common market-data adapter trait | [`crates/of_adapters/src/rithmic.rs:225`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L225) | `present` |
| `fn` | `from_config` | Creates a Rithmic adapter instance from generic adapter configuration | [`crates/of_adapters/src/rithmic.rs:245`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L245) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `variant` | `SequenceStatus` | `Ok` | `Ok` | [`crates/of_adapters/src/cqg/book.rs:33`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L33) |
| `variant` | `SequenceStatus` | `OutOfOrder` | `OutOfOrder` | [`crates/of_adapters/src/cqg/book.rs:34`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L34) |
| `variant` | `SequenceStatus` | `Gap` | `Gap { expected: u64, actual: u64 }` | [`crates/of_adapters/src/cqg/book.rs:35`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/book.rs#L35) |
| `field` | `CqgConfig` | `endpoint` | `: String` | [`crates/of_adapters/src/cqg/config.rs:7`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L7) |
| `field` | `CqgConfig` | `private_label` | `: String` | [`crates/of_adapters/src/cqg/config.rs:9`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L9) |
| `field` | `CqgConfig` | `client_id` | `: String` | [`crates/of_adapters/src/cqg/config.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L11) |
| `field` | `CqgConfig` | `username` | `: String` | [`crates/of_adapters/src/cqg/config.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L13) |
| `field` | `CqgConfig` | `password` | `: String` | [`crates/of_adapters/src/cqg/config.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L15) |
| `field` | `CqgConfig` | `ping_interval_secs` | `: u64` | [`crates/of_adapters/src/cqg/config.rs:17`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L17) |
| `field` | `CqgConfig` | `heartbeat_timeout_secs` | `: u64` | [`crates/of_adapters/src/cqg/config.rs:19`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L19) |
| `field` | `CqgConfig` | `reconnect_min_ms` | `: u64` | [`crates/of_adapters/src/cqg/config.rs:21`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L21) |
| `field` | `CqgConfig` | `reconnect_max_ms` | `: u64` | [`crates/of_adapters/src/cqg/config.rs:23`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L23) |
| `field` | `CqgConfig` | `max_inflight_requests` | `: u32` | [`crates/of_adapters/src/cqg/config.rs:25`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/config.rs#L25) |
| `field` | `CqgMetrics` | `ws_connect_attempts` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:4`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L4) |
| `field` | `CqgMetrics` | `ws_connect_failures` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:5`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L5) |
| `field` | `CqgMetrics` | `logon_success` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:6`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L6) |
| `field` | `CqgMetrics` | `logon_reject` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:7`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L7) |
| `field` | `CqgMetrics` | `symbol_resolve_success` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:8`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L8) |
| `field` | `CqgMetrics` | `symbol_resolve_fail` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:9`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L9) |
| `field` | `CqgMetrics` | `md_subscribe_success` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:10`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L10) |
| `field` | `CqgMetrics` | `md_subscribe_fail` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L11) |
| `field` | `CqgMetrics` | `md_subscribe_ack_mismatch` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:12`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L12) |
| `field` | `CqgMetrics` | `decode_errors` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L13) |
| `field` | `CqgMetrics` | `sequence_gaps` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:14`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L14) |
| `field` | `CqgMetrics` | `reconnect_count` | `: u64` | [`crates/of_adapters/src/cqg/metrics.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/metrics.rs#L15) |
| `variant` | `CqgOutbound` | `Logon` | `Logon` | [`crates/of_adapters/src/cqg/proto.rs:4`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L4) |
| `variant` | `CqgOutbound` | `Ping` | `Ping` | [`crates/of_adapters/src/cqg/proto.rs:14`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L14) |
| `variant` | `CqgOutbound` | `Logoff` | `Logoff` | [`crates/of_adapters/src/cqg/proto.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L15) |
| `variant` | `CqgInbound` | `Heartbeat` | `Heartbeat` | [`crates/of_adapters/src/cqg/proto.rs:55`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/proto.rs#L55) |
| `variant` | `CqgSessionState` | `Disconnected` | `Disconnected` | [`crates/of_adapters/src/cqg/session.rs:8`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L8) |
| `variant` | `CqgSessionState` | `Connecting` | `Connecting` | [`crates/of_adapters/src/cqg/session.rs:9`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L9) |
| `variant` | `CqgSessionState` | `LogonPending` | `LogonPending` | [`crates/of_adapters/src/cqg/session.rs:10`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L10) |
| `variant` | `CqgSessionState` | `ResolvingSymbols` | `ResolvingSymbols` | [`crates/of_adapters/src/cqg/session.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L11) |
| `variant` | `CqgSessionState` | `Subscribing` | `Subscribing` | [`crates/of_adapters/src/cqg/session.rs:12`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L12) |
| `variant` | `CqgSessionState` | `Streaming` | `Streaming` | [`crates/of_adapters/src/cqg/session.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L13) |
| `variant` | `CqgSessionState` | `Degraded` | `Degraded` | [`crates/of_adapters/src/cqg/session.rs:14`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L14) |
| `variant` | `CqgSessionState` | `BackoffWait` | `BackoffWait` | [`crates/of_adapters/src/cqg/session.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L15) |
| `field` | `CqgSession` | `symbol_to_contract` | `: HashMap<SymbolId, i64>` | [`crates/of_adapters/src/cqg/session.rs:23`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L23) |
| `field` | `CqgSession` | `requested_depth` | `: HashMap<SymbolId, u16>` | [`crates/of_adapters/src/cqg/session.rs:26`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/session.rs#L26) |
| `field` | `MockTransport` | `sent_frames` | `: Vec<Vec<u8>>` | [`crates/of_adapters/src/cqg/transport.rs:25`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/cqg/transport.rs#L25) |
| `field` | `SubscribeReq` | `symbol` | `: SymbolId` | [`crates/of_adapters/src/lib.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L15) |
| `field` | `SubscribeReq` | `depth_levels` | `: u16` | [`crates/of_adapters/src/lib.rs:17`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L17) |
| `field` | `AdapterHealth` | `connected` | `: bool` | [`crates/of_adapters/src/lib.rs:24`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L24) |
| `field` | `AdapterHealth` | `degraded` | `: bool` | [`crates/of_adapters/src/lib.rs:26`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L26) |
| `field` | `AdapterHealth` | `last_error` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:28`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L28) |
| `field` | `AdapterHealth` | `protocol_info` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:30`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L30) |
| `variant` | `AdapterRuntimeMode` | `Mock` | `Mock` | [`crates/of_adapters/src/lib.rs:38`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L38) |
| `variant` | `AdapterRuntimeMode` | `Live` | `Live` | [`crates/of_adapters/src/lib.rs:40`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L40) |
| `variant` | `AdapterRuntimeMode` | `Replay` | `Replay` | [`crates/of_adapters/src/lib.rs:42`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L42) |
| `variant` | `AdapterRuntimeMode` | `Bridge` | `Bridge` | [`crates/of_adapters/src/lib.rs:44`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L44) |
| `variant` | `AdapterRuntimeMode` | `Unknown` | `Unknown` | [`crates/of_adapters/src/lib.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L47) |
| `variant` | `AdapterConnectionState` | `Disconnected` | `Disconnected` | [`crates/of_adapters/src/lib.rs:68`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L68) |
| `variant` | `AdapterConnectionState` | `Connecting` | `Connecting` | [`crates/of_adapters/src/lib.rs:70`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L70) |
| `variant` | `AdapterConnectionState` | `Streaming` | `Streaming` | [`crates/of_adapters/src/lib.rs:72`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L72) |
| `variant` | `AdapterConnectionState` | `Reconnecting` | `Reconnecting` | [`crates/of_adapters/src/lib.rs:74`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L74) |
| `variant` | `AdapterConnectionState` | `Backoff` | `Backoff` | [`crates/of_adapters/src/lib.rs:76`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L76) |
| `variant` | `AdapterConnectionState` | `Replay` | `Replay` | [`crates/of_adapters/src/lib.rs:78`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L78) |
| `variant` | `AdapterConnectionState` | `Unknown` | `Unknown` | [`crates/of_adapters/src/lib.rs:81`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L81) |
| `field` | `AdapterOperationalStatus` | `mode` | `: AdapterRuntimeMode` | [`crates/of_adapters/src/lib.rs:110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L110) |
| `field` | `AdapterOperationalStatus` | `connection_state` | `: AdapterConnectionState` | [`crates/of_adapters/src/lib.rs:112`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L112) |
| `field` | `AdapterOperationalStatus` | `endpoint_redacted` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:114`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L114) |
| `field` | `AdapterOperationalStatus` | `app_name` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:116`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L116) |
| `field` | `AdapterOperationalStatus` | `reconnect_attempt` | `: u32` | [`crates/of_adapters/src/lib.rs:118`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L118) |
| `field` | `AdapterOperationalStatus` | `subscription_count` | `: usize` | [`crates/of_adapters/src/lib.rs:120`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L120) |
| `field` | `AdapterOperationalStatus` | `subscribed_symbols` | `: Vec<SymbolId>` | [`crates/of_adapters/src/lib.rs:122`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L122) |
| `field` | `AdapterOperationalStatus` | `queue_depth` | `: usize` | [`crates/of_adapters/src/lib.rs:124`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L124) |
| `field` | `AdapterOperationalStatus` | `queue_capacity` | `: Option<usize>` | [`crates/of_adapters/src/lib.rs:126`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L126) |
| `field` | `AdapterOperationalStatus` | `dropped_events` | `: u64` | [`crates/of_adapters/src/lib.rs:128`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L128) |
| `field` | `AdapterOperationalStatus` | `gap_count` | `: u64` | [`crates/of_adapters/src/lib.rs:130`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L130) |
| `field` | `AdapterOperationalStatus` | `stale` | `: bool` | [`crates/of_adapters/src/lib.rs:132`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L132) |
| `field` | `AdapterOperationalStatus` | `raw_capture_enabled` | `: bool` | [`crates/of_adapters/src/lib.rs:134`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L134) |
| `field` | `AdapterOperationalStatus` | `raw_capture_depth` | `: usize` | [`crates/of_adapters/src/lib.rs:136`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L136) |
| `field` | `AdapterOperationalStatus` | `raw_capture_capacity` | `: usize` | [`crates/of_adapters/src/lib.rs:138`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L138) |
| `field` | `AdapterOperationalStatus` | `last_message_age_ms` | `: Option<u64>` | [`crates/of_adapters/src/lib.rs:140`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L140) |
| `field` | `AdapterOperationalStatus` | `last_market_data_age_ms` | `: Option<u64>` | [`crates/of_adapters/src/lib.rs:142`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L142) |
| `variant` | `RawEvent` | `Book` | `Book(BookUpdate)` | [`crates/of_adapters/src/lib.rs:276`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L276) |
| `variant` | `RawEvent` | `Trade` | `Trade(TradePrint)` | [`crates/of_adapters/src/lib.rs:278`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L278) |
| `variant` | `AdapterError` | `Disconnected` | `Disconnected` | [`crates/of_adapters/src/lib.rs:285`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L285) |
| `variant` | `AdapterError` | `NotConfigured` | `NotConfigured(&'static str)` | [`crates/of_adapters/src/lib.rs:287`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L287) |
| `variant` | `AdapterError` | `FeatureDisabled` | `FeatureDisabled(&'static str)` | [`crates/of_adapters/src/lib.rs:289`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L289) |
| `variant` | `AdapterError` | `Other` | `Other(String)` | [`crates/of_adapters/src/lib.rs:291`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L291) |
| `variant` | `ProviderKind` | `Mock` | `Mock` | [`crates/of_adapters/src/lib.rs:361`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L361) |
| `variant` | `ProviderKind` | `Rithmic` | `Rithmic` | [`crates/of_adapters/src/lib.rs:363`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L363) |
| `variant` | `ProviderKind` | `Cqg` | `Cqg` | [`crates/of_adapters/src/lib.rs:365`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L365) |
| `variant` | `ProviderKind` | `Binance` | `Binance` | [`crates/of_adapters/src/lib.rs:367`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L367) |
| `variant` | `AdapterQualityLevel` | `Experimental` | `Experimental` | [`crates/of_adapters/src/lib.rs:387`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L387) |
| `variant` | `AdapterQualityLevel` | `Simulation` | `Simulation` | [`crates/of_adapters/src/lib.rs:389`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L389) |
| `variant` | `AdapterQualityLevel` | `Scaffold` | `Scaffold` | [`crates/of_adapters/src/lib.rs:391`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L391) |
| `variant` | `AdapterQualityLevel` | `SimulatedCertified` | `SimulatedCertified` | [`crates/of_adapters/src/lib.rs:393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L393) |
| `variant` | `AdapterQualityLevel` | `Functional` | `Functional` | [`crates/of_adapters/src/lib.rs:395`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L395) |
| `variant` | `AdapterQualityLevel` | `PaperTrading` | `PaperTrading` | [`crates/of_adapters/src/lib.rs:397`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L397) |
| `variant` | `AdapterQualityLevel` | `ProductionCandidate` | `ProductionCandidate` | [`crates/of_adapters/src/lib.rs:399`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L399) |
| `variant` | `AdapterQualityLevel` | `Certified` | `Certified` | [`crates/of_adapters/src/lib.rs:401`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L401) |
| `variant` | `AdapterQualityLevel` | `ProductionObserved` | `ProductionObserved` | [`crates/of_adapters/src/lib.rs:403`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L403) |
| `field` | `AdapterDescriptor` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:448`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L448) |
| `field` | `AdapterDescriptor` | `provider_id` | `: &'static str` | [`crates/of_adapters/src/lib.rs:450`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L450) |
| `field` | `AdapterDescriptor` | `display_name` | `: &'static str` | [`crates/of_adapters/src/lib.rs:452`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L452) |
| `field` | `AdapterDescriptor` | `feature` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:454`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L454) |
| `field` | `AdapterDescriptor` | `compiled` | `: bool` | [`crates/of_adapters/src/lib.rs:456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L456) |
| `field` | `AdapterDescriptor` | `quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:458`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L458) |
| `field` | `AdapterDescriptor` | `supports_live` | `: bool` | [`crates/of_adapters/src/lib.rs:460`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L460) |
| `field` | `AdapterDescriptor` | `supports_replay` | `: bool` | [`crates/of_adapters/src/lib.rs:462`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L462) |
| `field` | `AdapterDescriptor` | `supports_trades` | `: bool` | [`crates/of_adapters/src/lib.rs:464`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L464) |
| `field` | `AdapterDescriptor` | `supports_order_book` | `: bool` | [`crates/of_adapters/src/lib.rs:466`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L466) |
| `field` | `AdapterDescriptor` | `supports_level2` | `: bool` | [`crates/of_adapters/src/lib.rs:468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L468) |
| `field` | `AdapterDescriptor` | `supports_reconnect` | `: bool` | [`crates/of_adapters/src/lib.rs:470`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L470) |
| `field` | `AdapterDescriptor` | `supports_gap_recovery` | `: bool` | [`crates/of_adapters/src/lib.rs:472`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L472) |
| `field` | `AdapterDescriptor` | `supports_backpressure` | `: bool` | [`crates/of_adapters/src/lib.rs:474`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L474) |
| `field` | `AdapterDescriptor` | `supports_raw_capture` | `: bool` | [`crates/of_adapters/src/lib.rs:476`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L476) |
| `field` | `AdapterDescriptor` | `supports_fixture_replay` | `: bool` | [`crates/of_adapters/src/lib.rs:478`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L478) |
| `field` | `AdapterDescriptor` | `supports_stale_detection` | `: bool` | [`crates/of_adapters/src/lib.rs:480`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L480) |
| `field` | `AdapterDescriptor` | `supports_latency_metrics` | `: bool` | [`crates/of_adapters/src/lib.rs:482`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L482) |
| `field` | `AdapterDescriptor` | `supports_polling` | `: bool` | [`crates/of_adapters/src/lib.rs:484`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L484) |
| `field` | `AdapterDescriptor` | `certification_evidence` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:486`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L486) |
| `field` | `AdapterDescriptor` | `production_evidence` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:488`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L488) |
| `field` | `AdapterDescriptor` | `notes` | `: &'static str` | [`crates/of_adapters/src/lib.rs:490`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L490) |
| `variant` | `AdapterConformanceRequirement` | `AdvertisedQuality` | `AdvertisedQuality` | [`crates/of_adapters/src/lib.rs:498`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L498) |
| `variant` | `AdapterConformanceRequirement` | `Compiled` | `Compiled` | [`crates/of_adapters/src/lib.rs:500`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L500) |
| `variant` | `AdapterConformanceRequirement` | `LiveEndpoint` | `LiveEndpoint` | [`crates/of_adapters/src/lib.rs:502`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L502) |
| `variant` | `AdapterConformanceRequirement` | `ReplayOrSimulation` | `ReplayOrSimulation` | [`crates/of_adapters/src/lib.rs:504`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L504) |
| `variant` | `AdapterConformanceRequirement` | `MarketDataEvents` | `MarketDataEvents` | [`crates/of_adapters/src/lib.rs:506`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L506) |
| `variant` | `AdapterConformanceRequirement` | `PollingContract` | `PollingContract` | [`crates/of_adapters/src/lib.rs:508`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L508) |
| `variant` | `AdapterConformanceRequirement` | `Reconnect` | `Reconnect` | [`crates/of_adapters/src/lib.rs:510`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L510) |
| `variant` | `AdapterConformanceRequirement` | `GapRecovery` | `GapRecovery` | [`crates/of_adapters/src/lib.rs:512`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L512) |
| `variant` | `AdapterConformanceRequirement` | `Backpressure` | `Backpressure` | [`crates/of_adapters/src/lib.rs:514`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L514) |
| `variant` | `AdapterConformanceRequirement` | `StaleDetection` | `StaleDetection` | [`crates/of_adapters/src/lib.rs:516`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L516) |
| `variant` | `AdapterConformanceRequirement` | `LatencyMetrics` | `LatencyMetrics` | [`crates/of_adapters/src/lib.rs:518`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L518) |
| `variant` | `AdapterConformanceRequirement` | `RawCapture` | `RawCapture` | [`crates/of_adapters/src/lib.rs:520`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L520) |
| `variant` | `AdapterConformanceRequirement` | `FixtureReplay` | `FixtureReplay` | [`crates/of_adapters/src/lib.rs:522`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L522) |
| `variant` | `AdapterConformanceRequirement` | `CertificationEvidence` | `CertificationEvidence` | [`crates/of_adapters/src/lib.rs:524`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L524) |
| `variant` | `AdapterConformanceRequirement` | `ProductionEvidence` | `ProductionEvidence` | [`crates/of_adapters/src/lib.rs:526`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L526) |
| `field` | `AdapterConformanceFailure` | `requirement` | `: AdapterConformanceRequirement` | [`crates/of_adapters/src/lib.rs:556`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L556) |
| `field` | `AdapterConformanceFailure` | `message` | `: &'static str` | [`crates/of_adapters/src/lib.rs:558`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L558) |
| `field` | `AdapterConformanceReport` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:565`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L565) |
| `field` | `AdapterConformanceReport` | `provider_id` | `: &'static str` | [`crates/of_adapters/src/lib.rs:567`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L567) |
| `field` | `AdapterConformanceReport` | `advertised_quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:569`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L569) |
| `field` | `AdapterConformanceReport` | `target_quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:571`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L571) |
| `field` | `AdapterConformanceReport` | `checked_requirements` | `: usize` | [`crates/of_adapters/src/lib.rs:573`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L573) |
| `field` | `AdapterConformanceReport` | `failures` | `: Vec<AdapterConformanceFailure>` | [`crates/of_adapters/src/lib.rs:575`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L575) |
| `field` | `AdapterConfig` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:904`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L904) |
| `field` | `AdapterConfig` | `credentials` | `: Option<CredentialsRef>` | [`crates/of_adapters/src/lib.rs:906`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L906) |
| `field` | `AdapterConfig` | `endpoint` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:908`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L908) |
| `field` | `AdapterConfig` | `app_name` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:910`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L910) |
| `field` | `CredentialsRef` | `key_id_env` | `: String` | [`crates/of_adapters/src/lib.rs:928`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L928) |
| `field` | `CredentialsRef` | `secret_env` | `: String` | [`crates/of_adapters/src/lib.rs:930`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L930) |
| `field` | `MockAdapter` | `connected` | `: bool` | [`crates/of_adapters/src/lib.rs:1108`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1108) |
| `field` | `MockAdapter` | `subscribed` | `: Vec<SubscribeReq>` | [`crates/of_adapters/src/lib.rs:1110`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1110) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
