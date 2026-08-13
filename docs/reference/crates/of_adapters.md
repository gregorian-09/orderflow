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
| `struct` | `BinanceAdapter` | Binance websocket adapter with mock/live transport support | [`crates/of_adapters/src/binance.rs:242`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L242) | `present` |
| `fn` | `from_config` | Creates a Binance adapter from generic adapter configuration | [`crates/of_adapters/src/binance.rs:282`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L282) | `present` |
| `fn` | `with_max_queue_depth` | Returns a copy of this adapter with a maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:334`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L334) | `present` |
| `fn` | `set_max_queue_depth` | Sets the maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:343`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L343) | `present` |
| `fn` | `max_queue_depth` | Returns the configured maximum pending event queue depth | [`crates/of_adapters/src/binance.rs:351`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L351) | `present` |
| `fn` | `with_raw_capture_capacity` | Returns a copy of this adapter with raw inbound message capture enabled | [`crates/of_adapters/src/binance.rs:360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L360) | `present` |
| `fn` | `set_raw_capture_capacity` | Sets the raw inbound message capture capacity | [`crates/of_adapters/src/binance.rs:369`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L369) | `present` |
| `fn` | `raw_capture_capacity` | Returns the configured raw inbound message capture capacity | [`crates/of_adapters/src/binance.rs:384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L384) | `present` |
| `fn` | `raw_capture_len` | Returns the number of raw inbound messages currently buffered | [`crates/of_adapters/src/binance.rs:389`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L389) | `present` |
| `fn` | `raw_capture_dropped` | Returns the number of raw inbound messages dropped from the capture ring | [`crates/of_adapters/src/binance.rs:394`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L394) | `present` |
| `fn` | `drain_raw_messages` | Drains captured raw inbound messages into `out` | [`crates/of_adapters/src/binance.rs:402`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L402) | `present` |
| `fn` | `replay_raw_messages` | Replays raw Binance JSON messages into normalized events | [`crates/of_adapters/src/binance.rs:415`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/binance.rs#L415) | `present` |
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
| `struct` | `SubscribeReq` | Subscription request forwarded to adapters | [`crates/of_adapters/src/lib.rs:10`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L10) | `present` |
| `struct` | `AdapterHealth` | Adapter connection and quality health snapshot | [`crates/of_adapters/src/lib.rs:19`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L19) | `present` |
| `enum` | `AdapterRuntimeMode` | Transport mode used by an active adapter instance | [`crates/of_adapters/src/lib.rs:33`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L33) | `present` |
| `fn` | `id` | Returns the stable lowercase identifier used in status payloads | [`crates/of_adapters/src/lib.rs:49`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L49) | `present` |
| `enum` | `AdapterConnectionState` | Provider connection state reported by an active adapter instance | [`crates/of_adapters/src/lib.rs:63`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L63) | `present` |
| `fn` | `id` | Returns the stable lowercase identifier used in status payloads | [`crates/of_adapters/src/lib.rs:83`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L83) | `present` |
| `struct` | `AdapterOperationalStatus` | Typed operational status for a market-data adapter | [`crates/of_adapters/src/lib.rs:105`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L105) | `present` |
| `fn` | `new` | Creates an operational snapshot with the supplied mode and state | [`crates/of_adapters/src/lib.rs:144`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L144) | `present` |
| `fn` | `with_mode` | Sets the transport mode | [`crates/of_adapters/src/lib.rs:153`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L153) | `present` |
| `fn` | `with_connection_state` | Sets the provider connection state | [`crates/of_adapters/src/lib.rs:159`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L159) | `present` |
| `fn` | `with_endpoint` | Redacts and sets a configured endpoint | [`crates/of_adapters/src/lib.rs:165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L165) | `present` |
| `fn` | `with_app_name` | Sets a non-secret provider application name | [`crates/of_adapters/src/lib.rs:171`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L171) | `present` |
| `fn` | `with_reconnect_attempt` | Sets the current reconnect attempt number | [`crates/of_adapters/src/lib.rs:177`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L177) | `present` |
| `fn` | `with_subscribed_symbols` | Sets and deterministically orders active subscriptions | [`crates/of_adapters/src/lib.rs:183`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L183) | `present` |
| `fn` | `with_queue` | Sets provider-event queue utilization | [`crates/of_adapters/src/lib.rs:197`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L197) | `present` |
| `fn` | `with_loss_counters` | Sets drop and sequence-gap counters | [`crates/of_adapters/src/lib.rs:204`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L204) | `present` |
| `fn` | `with_stale` | Sets the adapter-specific stale-feed state | [`crates/of_adapters/src/lib.rs:211`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L211) | `present` |
| `fn` | `with_raw_capture` | Sets bounded raw-message capture utilization | [`crates/of_adapters/src/lib.rs:217`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L217) | `present` |
| `fn` | `with_activity_ages` | Sets provider-message and normalized-event ages | [`crates/of_adapters/src/lib.rs:225`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L225) | `present` |
| `fn` | `redact_adapter_endpoint` | Returns a diagnostics-safe endpoint containing only URI scheme and authority | [`crates/of_adapters/src/lib.rs:240`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L240) | `present` |
| `enum` | `RawEvent` | Raw adapter event stream | [`crates/of_adapters/src/lib.rs:271`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L271) | `present` |
| `enum` | `AdapterError` | Adapter-level error variants | [`crates/of_adapters/src/lib.rs:280`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L280) | `present` |
| `type` | `AdapterResult` | Result type alias used by adapter interfaces | [`crates/of_adapters/src/lib.rs:305`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L305) | `present` |
| `trait` | `MarketDataAdapter` | Common market-data adapter interface used by runtime | [`crates/of_adapters/src/lib.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L308) | `present` |
| `enum` | `ProviderKind` | Provider selection used by adapter factory configuration | [`crates/of_adapters/src/lib.rs:356`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L356) | `present` |
| `fn` | `id` | Returns the stable lowercase provider id used in diagnostics | [`crates/of_adapters/src/lib.rs:369`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L369) | `present` |
| `enum` | `AdapterQualityLevel` | Adapter maturity level advertised by the discovery registry | [`crates/of_adapters/src/lib.rs:382`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L382) | `present` |
| `fn` | `id` | Returns the stable lowercase quality id used in diagnostics | [`crates/of_adapters/src/lib.rs:405`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L405) | `present` |
| `fn` | `rank` | Returns the conservative ordering used by conformance reports | [`crates/of_adapters/src/lib.rs:420`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L420) | `present` |
| `fn` | `meets` | Returns true when this level is at least as mature as `target` | [`crates/of_adapters/src/lib.rs:435`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L435) | `present` |
| `struct` | `AdapterDescriptor` | Static capability description for one market-data adapter | [`crates/of_adapters/src/lib.rs:443`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L443) | `present` |
| `enum` | `AdapterConformanceRequirement` | Adapter conformance requirement checked for a target quality level | [`crates/of_adapters/src/lib.rs:493`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L493) | `present` |
| `fn` | `id` | Returns the stable lowercase requirement id used in reports | [`crates/of_adapters/src/lib.rs:528`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L528) | `present` |
| `struct` | `AdapterConformanceFailure` | One failed adapter conformance requirement | [`crates/of_adapters/src/lib.rs:551`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L551) | `present` |
| `struct` | `AdapterConformanceReport` | Adapter conformance report for one descriptor and target quality level | [`crates/of_adapters/src/lib.rs:560`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L560) | `present` |
| `fn` | `passed` | Returns true when every checked requirement passed | [`crates/of_adapters/src/lib.rs:577`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L577) | `present` |
| `fn` | `adapter_quality_requirements` | Returns the conformance requirements for a target adapter quality level | [`crates/of_adapters/src/lib.rs:649`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L649) | `present` |
| `fn` | `adapter_descriptors` | Returns static descriptors for all known adapter providers | [`crates/of_adapters/src/lib.rs:767`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L767) | `present` |
| `fn` | `compiled_adapter_descriptors` | Returns descriptors for providers compiled into the current binary | [`crates/of_adapters/src/lib.rs:772`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L772) | `present` |
| `fn` | `describe_adapter` | Returns the descriptor for `provider` | [`crates/of_adapters/src/lib.rs:781`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L781) | `present` |
| `fn` | `adapter_feature_enabled` | Returns true when the current binary can construct `provider` | [`crates/of_adapters/src/lib.rs:790`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L790) | `present` |
| `fn` | `evaluate_adapter_conformance` | Evaluates whether a descriptor satisfies a target adapter quality level | [`crates/of_adapters/src/lib.rs:798`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L798) | `present` |
| `fn` | `adapter_conformance_report` | Evaluates a known provider against a target adapter quality level | [`crates/of_adapters/src/lib.rs:832`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L832) | `present` |
| `struct` | `AdapterConfig` | Generic adapter factory configuration | [`crates/of_adapters/src/lib.rs:899`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L899) | `present` |
| `struct` | `CredentialsRef` | Credential environment-variable references for adapter auth bootstrap | [`crates/of_adapters/src/lib.rs:923`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L923) | `present` |
| `fn` | `create_adapter` | Creates a provider adapter from configuration | [`crates/of_adapters/src/lib.rs:931`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L931) | `present` |
| `struct` | `MockAdapter` | Deterministic in-memory adapter for tests, demos, and replay harnesses | [`crates/of_adapters/src/lib.rs:990`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L990) | `present` |
| `fn` | `push_event` | Pushes an event into mock queue, drained by `poll` | [`crates/of_adapters/src/lib.rs:1000`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1000) | `present` |
| `mod` | `rithmic` | Rithmic adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1059`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1059) | `present` |
| `mod` | `cqg` | CQG adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1063`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1063) | `present` |
| `mod` | `binance` | Binance adapter implementation (feature-gated) | [`crates/of_adapters/src/lib.rs:1067`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L1067) | `present` |
| `struct` | `RithmicConfig` | Resolved runtime configuration for the feature-gated Rithmic adapter | [`crates/of_adapters/src/rithmic.rs:19`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L19) | `present` |
| `fn` | `from_adapter_config` | Builds a validated Rithmic config from generic adapter config input | [`crates/of_adapters/src/rithmic.rs:28`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L28) | `present` |
| `struct` | `RithmicAdapter` | Rithmic adapter implementing the common market-data adapter trait | [`crates/of_adapters/src/rithmic.rs:230`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L230) | `present` |
| `fn` | `from_config` | Creates a Rithmic adapter instance from generic adapter configuration | [`crates/of_adapters/src/rithmic.rs:250`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/rithmic.rs#L250) | `present` |

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
| `field` | `SubscribeReq` | `symbol` | `: SymbolId` | [`crates/of_adapters/src/lib.rs:12`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L12) |
| `field` | `SubscribeReq` | `depth_levels` | `: u16` | [`crates/of_adapters/src/lib.rs:14`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L14) |
| `field` | `AdapterHealth` | `connected` | `: bool` | [`crates/of_adapters/src/lib.rs:21`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L21) |
| `field` | `AdapterHealth` | `degraded` | `: bool` | [`crates/of_adapters/src/lib.rs:23`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L23) |
| `field` | `AdapterHealth` | `last_error` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:25`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L25) |
| `field` | `AdapterHealth` | `protocol_info` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:27`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L27) |
| `variant` | `AdapterRuntimeMode` | `Mock` | `Mock` | [`crates/of_adapters/src/lib.rs:35`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L35) |
| `variant` | `AdapterRuntimeMode` | `Live` | `Live` | [`crates/of_adapters/src/lib.rs:37`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L37) |
| `variant` | `AdapterRuntimeMode` | `Replay` | `Replay` | [`crates/of_adapters/src/lib.rs:39`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L39) |
| `variant` | `AdapterRuntimeMode` | `Bridge` | `Bridge` | [`crates/of_adapters/src/lib.rs:41`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L41) |
| `variant` | `AdapterRuntimeMode` | `Unknown` | `Unknown` | [`crates/of_adapters/src/lib.rs:44`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L44) |
| `variant` | `AdapterConnectionState` | `Disconnected` | `Disconnected` | [`crates/of_adapters/src/lib.rs:65`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L65) |
| `variant` | `AdapterConnectionState` | `Connecting` | `Connecting` | [`crates/of_adapters/src/lib.rs:67`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L67) |
| `variant` | `AdapterConnectionState` | `Streaming` | `Streaming` | [`crates/of_adapters/src/lib.rs:69`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L69) |
| `variant` | `AdapterConnectionState` | `Reconnecting` | `Reconnecting` | [`crates/of_adapters/src/lib.rs:71`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L71) |
| `variant` | `AdapterConnectionState` | `Backoff` | `Backoff` | [`crates/of_adapters/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L73) |
| `variant` | `AdapterConnectionState` | `Replay` | `Replay` | [`crates/of_adapters/src/lib.rs:75`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L75) |
| `variant` | `AdapterConnectionState` | `Unknown` | `Unknown` | [`crates/of_adapters/src/lib.rs:78`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L78) |
| `field` | `AdapterOperationalStatus` | `mode` | `: AdapterRuntimeMode` | [`crates/of_adapters/src/lib.rs:107`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L107) |
| `field` | `AdapterOperationalStatus` | `connection_state` | `: AdapterConnectionState` | [`crates/of_adapters/src/lib.rs:109`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L109) |
| `field` | `AdapterOperationalStatus` | `endpoint_redacted` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:111`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L111) |
| `field` | `AdapterOperationalStatus` | `app_name` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:113`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L113) |
| `field` | `AdapterOperationalStatus` | `reconnect_attempt` | `: u32` | [`crates/of_adapters/src/lib.rs:115`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L115) |
| `field` | `AdapterOperationalStatus` | `subscription_count` | `: usize` | [`crates/of_adapters/src/lib.rs:117`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L117) |
| `field` | `AdapterOperationalStatus` | `subscribed_symbols` | `: Vec<SymbolId>` | [`crates/of_adapters/src/lib.rs:119`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L119) |
| `field` | `AdapterOperationalStatus` | `queue_depth` | `: usize` | [`crates/of_adapters/src/lib.rs:121`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L121) |
| `field` | `AdapterOperationalStatus` | `queue_capacity` | `: Option<usize>` | [`crates/of_adapters/src/lib.rs:123`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L123) |
| `field` | `AdapterOperationalStatus` | `dropped_events` | `: u64` | [`crates/of_adapters/src/lib.rs:125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L125) |
| `field` | `AdapterOperationalStatus` | `gap_count` | `: u64` | [`crates/of_adapters/src/lib.rs:127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L127) |
| `field` | `AdapterOperationalStatus` | `stale` | `: bool` | [`crates/of_adapters/src/lib.rs:129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L129) |
| `field` | `AdapterOperationalStatus` | `raw_capture_enabled` | `: bool` | [`crates/of_adapters/src/lib.rs:131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L131) |
| `field` | `AdapterOperationalStatus` | `raw_capture_depth` | `: usize` | [`crates/of_adapters/src/lib.rs:133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L133) |
| `field` | `AdapterOperationalStatus` | `raw_capture_capacity` | `: usize` | [`crates/of_adapters/src/lib.rs:135`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L135) |
| `field` | `AdapterOperationalStatus` | `last_message_age_ms` | `: Option<u64>` | [`crates/of_adapters/src/lib.rs:137`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L137) |
| `field` | `AdapterOperationalStatus` | `last_market_data_age_ms` | `: Option<u64>` | [`crates/of_adapters/src/lib.rs:139`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L139) |
| `variant` | `RawEvent` | `Book` | `Book(BookUpdate)` | [`crates/of_adapters/src/lib.rs:273`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L273) |
| `variant` | `RawEvent` | `Trade` | `Trade(TradePrint)` | [`crates/of_adapters/src/lib.rs:275`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L275) |
| `variant` | `AdapterError` | `Disconnected` | `Disconnected` | [`crates/of_adapters/src/lib.rs:282`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L282) |
| `variant` | `AdapterError` | `NotConfigured` | `NotConfigured(&'static str)` | [`crates/of_adapters/src/lib.rs:284`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L284) |
| `variant` | `AdapterError` | `FeatureDisabled` | `FeatureDisabled(&'static str)` | [`crates/of_adapters/src/lib.rs:286`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L286) |
| `variant` | `AdapterError` | `Other` | `Other(String)` | [`crates/of_adapters/src/lib.rs:288`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L288) |
| `variant` | `ProviderKind` | `Mock` | `Mock` | [`crates/of_adapters/src/lib.rs:358`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L358) |
| `variant` | `ProviderKind` | `Rithmic` | `Rithmic` | [`crates/of_adapters/src/lib.rs:360`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L360) |
| `variant` | `ProviderKind` | `Cqg` | `Cqg` | [`crates/of_adapters/src/lib.rs:362`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L362) |
| `variant` | `ProviderKind` | `Binance` | `Binance` | [`crates/of_adapters/src/lib.rs:364`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L364) |
| `variant` | `AdapterQualityLevel` | `Experimental` | `Experimental` | [`crates/of_adapters/src/lib.rs:384`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L384) |
| `variant` | `AdapterQualityLevel` | `Simulation` | `Simulation` | [`crates/of_adapters/src/lib.rs:386`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L386) |
| `variant` | `AdapterQualityLevel` | `Scaffold` | `Scaffold` | [`crates/of_adapters/src/lib.rs:388`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L388) |
| `variant` | `AdapterQualityLevel` | `SimulatedCertified` | `SimulatedCertified` | [`crates/of_adapters/src/lib.rs:390`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L390) |
| `variant` | `AdapterQualityLevel` | `Functional` | `Functional` | [`crates/of_adapters/src/lib.rs:392`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L392) |
| `variant` | `AdapterQualityLevel` | `PaperTrading` | `PaperTrading` | [`crates/of_adapters/src/lib.rs:394`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L394) |
| `variant` | `AdapterQualityLevel` | `ProductionCandidate` | `ProductionCandidate` | [`crates/of_adapters/src/lib.rs:396`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L396) |
| `variant` | `AdapterQualityLevel` | `Certified` | `Certified` | [`crates/of_adapters/src/lib.rs:398`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L398) |
| `variant` | `AdapterQualityLevel` | `ProductionObserved` | `ProductionObserved` | [`crates/of_adapters/src/lib.rs:400`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L400) |
| `field` | `AdapterDescriptor` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:445`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L445) |
| `field` | `AdapterDescriptor` | `provider_id` | `: &'static str` | [`crates/of_adapters/src/lib.rs:447`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L447) |
| `field` | `AdapterDescriptor` | `display_name` | `: &'static str` | [`crates/of_adapters/src/lib.rs:449`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L449) |
| `field` | `AdapterDescriptor` | `feature` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:451`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L451) |
| `field` | `AdapterDescriptor` | `compiled` | `: bool` | [`crates/of_adapters/src/lib.rs:453`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L453) |
| `field` | `AdapterDescriptor` | `quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:455`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L455) |
| `field` | `AdapterDescriptor` | `supports_live` | `: bool` | [`crates/of_adapters/src/lib.rs:457`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L457) |
| `field` | `AdapterDescriptor` | `supports_replay` | `: bool` | [`crates/of_adapters/src/lib.rs:459`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L459) |
| `field` | `AdapterDescriptor` | `supports_trades` | `: bool` | [`crates/of_adapters/src/lib.rs:461`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L461) |
| `field` | `AdapterDescriptor` | `supports_order_book` | `: bool` | [`crates/of_adapters/src/lib.rs:463`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L463) |
| `field` | `AdapterDescriptor` | `supports_level2` | `: bool` | [`crates/of_adapters/src/lib.rs:465`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L465) |
| `field` | `AdapterDescriptor` | `supports_reconnect` | `: bool` | [`crates/of_adapters/src/lib.rs:467`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L467) |
| `field` | `AdapterDescriptor` | `supports_gap_recovery` | `: bool` | [`crates/of_adapters/src/lib.rs:469`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L469) |
| `field` | `AdapterDescriptor` | `supports_backpressure` | `: bool` | [`crates/of_adapters/src/lib.rs:471`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L471) |
| `field` | `AdapterDescriptor` | `supports_raw_capture` | `: bool` | [`crates/of_adapters/src/lib.rs:473`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L473) |
| `field` | `AdapterDescriptor` | `supports_fixture_replay` | `: bool` | [`crates/of_adapters/src/lib.rs:475`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L475) |
| `field` | `AdapterDescriptor` | `supports_stale_detection` | `: bool` | [`crates/of_adapters/src/lib.rs:477`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L477) |
| `field` | `AdapterDescriptor` | `supports_latency_metrics` | `: bool` | [`crates/of_adapters/src/lib.rs:479`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L479) |
| `field` | `AdapterDescriptor` | `supports_polling` | `: bool` | [`crates/of_adapters/src/lib.rs:481`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L481) |
| `field` | `AdapterDescriptor` | `certification_evidence` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:483`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L483) |
| `field` | `AdapterDescriptor` | `production_evidence` | `: Option<&'static str>` | [`crates/of_adapters/src/lib.rs:485`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L485) |
| `field` | `AdapterDescriptor` | `notes` | `: &'static str` | [`crates/of_adapters/src/lib.rs:487`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L487) |
| `variant` | `AdapterConformanceRequirement` | `AdvertisedQuality` | `AdvertisedQuality` | [`crates/of_adapters/src/lib.rs:495`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L495) |
| `variant` | `AdapterConformanceRequirement` | `Compiled` | `Compiled` | [`crates/of_adapters/src/lib.rs:497`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L497) |
| `variant` | `AdapterConformanceRequirement` | `LiveEndpoint` | `LiveEndpoint` | [`crates/of_adapters/src/lib.rs:499`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L499) |
| `variant` | `AdapterConformanceRequirement` | `ReplayOrSimulation` | `ReplayOrSimulation` | [`crates/of_adapters/src/lib.rs:501`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L501) |
| `variant` | `AdapterConformanceRequirement` | `MarketDataEvents` | `MarketDataEvents` | [`crates/of_adapters/src/lib.rs:503`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L503) |
| `variant` | `AdapterConformanceRequirement` | `PollingContract` | `PollingContract` | [`crates/of_adapters/src/lib.rs:505`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L505) |
| `variant` | `AdapterConformanceRequirement` | `Reconnect` | `Reconnect` | [`crates/of_adapters/src/lib.rs:507`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L507) |
| `variant` | `AdapterConformanceRequirement` | `GapRecovery` | `GapRecovery` | [`crates/of_adapters/src/lib.rs:509`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L509) |
| `variant` | `AdapterConformanceRequirement` | `Backpressure` | `Backpressure` | [`crates/of_adapters/src/lib.rs:511`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L511) |
| `variant` | `AdapterConformanceRequirement` | `StaleDetection` | `StaleDetection` | [`crates/of_adapters/src/lib.rs:513`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L513) |
| `variant` | `AdapterConformanceRequirement` | `LatencyMetrics` | `LatencyMetrics` | [`crates/of_adapters/src/lib.rs:515`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L515) |
| `variant` | `AdapterConformanceRequirement` | `RawCapture` | `RawCapture` | [`crates/of_adapters/src/lib.rs:517`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L517) |
| `variant` | `AdapterConformanceRequirement` | `FixtureReplay` | `FixtureReplay` | [`crates/of_adapters/src/lib.rs:519`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L519) |
| `variant` | `AdapterConformanceRequirement` | `CertificationEvidence` | `CertificationEvidence` | [`crates/of_adapters/src/lib.rs:521`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L521) |
| `variant` | `AdapterConformanceRequirement` | `ProductionEvidence` | `ProductionEvidence` | [`crates/of_adapters/src/lib.rs:523`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L523) |
| `field` | `AdapterConformanceFailure` | `requirement` | `: AdapterConformanceRequirement` | [`crates/of_adapters/src/lib.rs:553`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L553) |
| `field` | `AdapterConformanceFailure` | `message` | `: &'static str` | [`crates/of_adapters/src/lib.rs:555`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L555) |
| `field` | `AdapterConformanceReport` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:562`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L562) |
| `field` | `AdapterConformanceReport` | `provider_id` | `: &'static str` | [`crates/of_adapters/src/lib.rs:564`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L564) |
| `field` | `AdapterConformanceReport` | `advertised_quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:566`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L566) |
| `field` | `AdapterConformanceReport` | `target_quality` | `: AdapterQualityLevel` | [`crates/of_adapters/src/lib.rs:568`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L568) |
| `field` | `AdapterConformanceReport` | `checked_requirements` | `: usize` | [`crates/of_adapters/src/lib.rs:570`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L570) |
| `field` | `AdapterConformanceReport` | `failures` | `: Vec<AdapterConformanceFailure>` | [`crates/of_adapters/src/lib.rs:572`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L572) |
| `field` | `AdapterConfig` | `provider` | `: ProviderKind` | [`crates/of_adapters/src/lib.rs:901`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L901) |
| `field` | `AdapterConfig` | `credentials` | `: Option<CredentialsRef>` | [`crates/of_adapters/src/lib.rs:903`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L903) |
| `field` | `AdapterConfig` | `endpoint` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:905`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L905) |
| `field` | `AdapterConfig` | `app_name` | `: Option<String>` | [`crates/of_adapters/src/lib.rs:907`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L907) |
| `field` | `CredentialsRef` | `key_id_env` | `: String` | [`crates/of_adapters/src/lib.rs:925`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L925) |
| `field` | `CredentialsRef` | `secret_env` | `: String` | [`crates/of_adapters/src/lib.rs:927`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L927) |
| `field` | `MockAdapter` | `connected` | `: bool` | [`crates/of_adapters/src/lib.rs:992`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L992) |
| `field` | `MockAdapter` | `subscribed` | `: Vec<SubscribeReq>` | [`crates/of_adapters/src/lib.rs:994`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_adapters/src/lib.rs#L994) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
