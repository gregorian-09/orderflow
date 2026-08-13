# `of_execution_core` Reference

> Generated from `crates/of_execution_core/Cargo.toml`, `rust-surface.md`, and `rust-values.md`.

**Version:** `0.2.0`<br>
**Description:** Low-latency execution domain model, order state machine, and risk primitives for Orderflow<br>
**Source:** [`crates/of_execution_core/src`](https://github.com/gregorian-09/orderflow/tree/main/crates/of_execution_core/src)<br>
**Generated Rustdoc:** [open `of_execution_core` Rustdoc](https://docs.rs/of_execution_core/0.2.0/of_execution_core/)

This page is the crate-level index. The source links and generated
Rustdoc are authoritative for exact signatures, conditional compilation,
multiline declarations, and implementation-specific detail.

## Features

- No crate-defined features.

## Local Dependencies

- No local workspace dependencies.

## Public Declaration Index

| Kind | Name | Summary | Source | Docs marker |
| --- | --- | --- | --- | --- |
| `const` | `EXECUTION_TEXT_CAP` | Maximum bytes stored in an execution diagnostic text field | [`crates/of_execution_core/src/lib.rs:9`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L9) | `present` |
| `const` | `EXECUTION_WAL_MAGIC` | Magic value written at the start of every execution WAL frame | [`crates/of_execution_core/src/lib.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L11) | `present` |
| `const` | `EXECUTION_WAL_VERSION` | Binary execution WAL frame version | [`crates/of_execution_core/src/lib.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L13) | `present` |
| `const` | `EXECUTION_WAL_HEADER_LEN` | Encoded execution WAL header length in bytes | [`crates/of_execution_core/src/lib.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L15) | `present` |
| `const` | `EXECUTION_WAL_MAX_PAYLOAD_LEN` | Maximum payload bytes accepted by the execution WAL frame helpers | [`crates/of_execution_core/src/lib.rs:17`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L17) | `present` |
| `struct` | `FixedAscii` | Fixed-size ASCII field used for low-allocation identifiers | [`crates/of_execution_core/src/lib.rs:26`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L26) | `present` |
| `fn` | `empty` | Creates an empty fixed ASCII value | [`crates/of_execution_core/src/lib.rs:33`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L33) | `present` |
| `fn` | `new` | Creates a fixed ASCII value from `value` | [`crates/of_execution_core/src/lib.rs:47`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L47) | `present` |
| `fn` | `as_str` | Returns the identifier as a string slice | [`crates/of_execution_core/src/lib.rs:67`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L67) | `present` |
| `fn` | `capacity` | Returns the fixed field capacity in bytes | [`crates/of_execution_core/src/lib.rs:73`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L73) | `present` |
| `fn` | `is_empty` | Returns true when the identifier is empty | [`crates/of_execution_core/src/lib.rs:78`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L78) | `present` |
| `type` | `ClientOrderId` | Client-assigned order identifier | [`crates/of_execution_core/src/lib.rs:116`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L116) | `present` |
| `type` | `VenueOrderId` | Venue-assigned order identifier | [`crates/of_execution_core/src/lib.rs:118`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L118) | `present` |
| `type` | `ExecutionId` | Venue execution/fill identifier | [`crates/of_execution_core/src/lib.rs:120`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L120) | `present` |
| `type` | `AccountId` | Trading account identifier | [`crates/of_execution_core/src/lib.rs:122`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L122) | `present` |
| `type` | `RouteId` | Execution route identifier | [`crates/of_execution_core/src/lib.rs:124`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L124) | `present` |
| `type` | `StrategyId` | Strategy identifier used for attribution | [`crates/of_execution_core/src/lib.rs:126`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L126) | `present` |
| `type` | `VenueId` | Venue identifier used by execution routing | [`crates/of_execution_core/src/lib.rs:128`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L128) | `present` |
| `type` | `InstrumentId` | Instrument identifier in venue/native format | [`crates/of_execution_core/src/lib.rs:130`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L130) | `present` |
| `type` | `ExecutionText` | Bounded diagnostic text | [`crates/of_execution_core/src/lib.rs:132`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L132) | `present` |
| `enum` | `ExecutionCoreError` | Execution-core error | [`crates/of_execution_core/src/lib.rs:136`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L136) | `present` |
| `enum` | `WalChecksumField` | Execution WAL record checksum category | [`crates/of_execution_core/src/lib.rs:173`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L173) | `present` |
| `enum` | `ExecutionWalError` | Error returned by execution WAL frame helpers | [`crates/of_execution_core/src/lib.rs:183`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L183) | `present` |
| `struct` | `WalSequence` | Monotonic execution WAL sequence number | [`crates/of_execution_core/src/lib.rs:308`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L308) | `present` |
| `fn` | `next` | Returns the next sequence using saturating arithmetic | [`crates/of_execution_core/src/lib.rs:312`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L312) | `present` |
| `struct` | `WalSegmentId` | Execution WAL segment identifier | [`crates/of_execution_core/src/lib.rs:320`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L320) | `present` |
| `enum` | `WalRecordKind` | Execution WAL record kind | [`crates/of_execution_core/src/lib.rs:326`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L326) | `present` |
| `enum` | `WalSyncPolicy` | Execution WAL durability policy | [`crates/of_execution_core/src/lib.rs:369`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L369) | `present` |
| `struct` | `WalRecordHeader` | Fixed-size execution WAL record header | [`crates/of_execution_core/src/lib.rs:387`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L387) | `present` |
| `fn` | `new` | Creates a header for `payload` | [`crates/of_execution_core/src/lib.rs:421`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L421) | `present` |
| `fn` | `with_flags` | Sets writer-defined flags and refreshes the header checksum | [`crates/of_execution_core/src/lib.rs:452`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L452) | `present` |
| `fn` | `with_hashes` | Sets route/account/symbol hashes and refreshes the header checksum | [`crates/of_execution_core/src/lib.rs:459`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L459) | `present` |
| `fn` | `with_previous_checksum` | Sets the previous checksum link and refreshes the header checksum | [`crates/of_execution_core/src/lib.rs:468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L468) | `present` |
| `fn` | `frame_len` | Returns total encoded frame length for this header | [`crates/of_execution_core/src/lib.rs:475`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L475) | `present` |
| `struct` | `WalRecordView` | Borrowed execution WAL record | [`crates/of_execution_core/src/lib.rs:515`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L515) | `present` |
| `fn` | `new` | Creates a borrowed WAL record and computes its checksums | [`crates/of_execution_core/src/lib.rs:529`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L529) | `present` |
| `fn` | `from_header` | Creates a borrowed WAL record from an existing header and payload | [`crates/of_execution_core/src/lib.rs:547`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L547) | `present` |
| `fn` | `encoded_len` | Returns total encoded frame length | [`crates/of_execution_core/src/lib.rs:562`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L562) | `present` |
| `fn` | `encode_into` | Encodes this record into `out` | [`crates/of_execution_core/src/lib.rs:572`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L572) | `present` |
| `fn` | `append_to` | Appends this record to `out` | [`crates/of_execution_core/src/lib.rs:586`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L586) | `present` |
| `fn` | `decode` | Decodes one record from the beginning of `bytes` | [`crates/of_execution_core/src/lib.rs:599`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L599) | `present` |
| `struct` | `WalReplayCursor` | Sequential borrowed replay cursor for execution WAL bytes | [`crates/of_execution_core/src/lib.rs:624`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L624) | `present` |
| `fn` | `new` | Creates a cursor over encoded WAL bytes | [`crates/of_execution_core/src/lib.rs:633`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L633) | `present` |
| `fn` | `with_strict_sequence` | Enables or disables contiguous sequence validation | [`crates/of_execution_core/src/lib.rs:643`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L643) | `present` |
| `fn` | `offset` | Returns the current byte offset | [`crates/of_execution_core/src/lib.rs:649`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L649) | `present` |
| `fn` | `remaining` | Returns the number of unread bytes | [`crates/of_execution_core/src/lib.rs:654`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L654) | `present` |
| `fn` | `next_record` | Decodes the next record | [`crates/of_execution_core/src/lib.rs:665`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L665) | `present` |
| `struct` | `WalIntegrityReport` | Integrity summary for encoded execution WAL bytes | [`crates/of_execution_core/src/lib.rs:712`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L712) | `present` |
| `fn` | `inspect` | Inspects encoded WAL bytes and returns a non-panicking integrity report | [`crates/of_execution_core/src/lib.rs:733`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L733) | `present` |
| `fn` | `execution_wal_checksum` | Returns the deterministic non-cryptographic checksum used by WAL frames | [`crates/of_execution_core/src/lib.rs:778`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L778) | `present` |
| `struct` | `ExecutionSymbol` | Execution symbol in venue-native format | [`crates/of_execution_core/src/lib.rs:890`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L890) | `present` |
| `fn` | `new` | Creates a symbol from ASCII venue and instrument identifiers | [`crates/of_execution_core/src/lib.rs:903`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L903) | `present` |
| `struct` | `OrderQty` | Integer-normalized order quantity | [`crates/of_execution_core/src/lib.rs:914`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L914) | `present` |
| `fn` | `new` | Creates a positive order quantity | [`crates/of_execution_core/src/lib.rs:922`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L922) | `present` |
| `struct` | `OrderPrice` | Integer-normalized order price | [`crates/of_execution_core/src/lib.rs:933`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L933) | `present` |
| `fn` | `new` | Creates a positive order price | [`crates/of_execution_core/src/lib.rs:941`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L941) | `present` |
| `enum` | `OrderSide` | Buy/sell order side | [`crates/of_execution_core/src/lib.rs:952`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L952) | `present` |
| `enum` | `OrderType` | Supported canonical order types | [`crates/of_execution_core/src/lib.rs:962`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L962) | `present` |
| `enum` | `TimeInForce` | Time-in-force policy | [`crates/of_execution_core/src/lib.rs:976`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L976) | `present` |
| `enum` | `OrderStatus` | FIX-style canonical order status | [`crates/of_execution_core/src/lib.rs:992`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L992) | `present` |
| `fn` | `is_terminal` | Returns true when no further venue activity is expected | [`crates/of_execution_core/src/lib.rs:1021`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1021) | `present` |
| `enum` | `ExecutionType` | Canonical execution report purpose | [`crates/of_execution_core/src/lib.rs:1032`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1032) | `present` |
| `struct` | `OrderRequest` | New order request | [`crates/of_execution_core/src/lib.rs:1064`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1064) | `present` |
| `fn` | `validate` | Validates basic order shape | [`crates/of_execution_core/src/lib.rs:1100`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1100) | `present` |
| `struct` | `CancelRequest` | Cancel request | [`crates/of_execution_core/src/lib.rs:1119`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1119) | `present` |
| `struct` | `AmendRequest` | Amend/cancel-replace request | [`crates/of_execution_core/src/lib.rs:1139`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1139) | `present` |
| `struct` | `ExecutionEvent` | Canonical execution event | [`crates/of_execution_core/src/lib.rs:1163`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1163) | `present` |
| `fn` | `accepted` | Creates an accepted event from a new order request | [`crates/of_execution_core/src/lib.rs:1204`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1204) | `present` |
| `fn` | `rejected` | Creates a structured local rejection event from a request | [`crates/of_execution_core/src/lib.rs:1228`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1228) | `present` |
| `struct` | `OrderState` | Current order state | [`crates/of_execution_core/src/lib.rs:1255`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1255) | `present` |
| `fn` | `pending_new` | Creates local pending-new state from a request | [`crates/of_execution_core/src/lib.rs:1286`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1286) | `present` |
| `struct` | `OrderStateMachine` | Deterministic order state machine | [`crates/of_execution_core/src/lib.rs:1307`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1307) | `present` |
| `fn` | `new` | Creates a state machine from an order request | [`crates/of_execution_core/src/lib.rs:1313`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1313) | `present` |
| `fn` | `state` | Returns the current order state | [`crates/of_execution_core/src/lib.rs:1320`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1320) | `present` |
| `fn` | `apply` | Applies an execution event | [`crates/of_execution_core/src/lib.rs:1330`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1330) | `present` |
| `enum` | `RiskRejectReason` | Structured risk rejection reason | [`crates/of_execution_core/src/lib.rs:1442`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1442) | `present` |
| `struct` | `RiskDecision` | Risk decision | [`crates/of_execution_core/src/lib.rs:1474`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1474) | `present` |
| `fn` | `allow` | Creates an allow decision | [`crates/of_execution_core/src/lib.rs:1485`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1485) | `present` |
| `fn` | `reject` | Creates a reject decision | [`crates/of_execution_core/src/lib.rs:1494`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1494) | `present` |
| `struct` | `RiskLimits` | Static risk limits for one route/account scope | [`crates/of_execution_core/src/lib.rs:1506`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1506) | `present` |
| `struct` | `RiskContext` | Runtime risk context supplied by the execution engine | [`crates/of_execution_core/src/lib.rs:1537`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1537) | `present` |
| `trait` | `RiskCheck` | Pre-trade risk-check contract | [`crates/of_execution_core/src/lib.rs:1575`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1575) | `present` |
| `struct` | `BasicRiskGate` | Deterministic pre-trade risk gate | [`crates/of_execution_core/src/lib.rs:1586`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1586) | `present` |
| `fn` | `new` | Creates a risk gate from static limits | [`crates/of_execution_core/src/lib.rs:1592`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1592) | `present` |

## Constants, Aliases, Fields, and Variants

| Kind | Owner | Name | Declared type/value | Source |
| --- | --- | --- | --- | --- |
| `const` | `-` | `EXECUTION_TEXT_CAP` | `: usize = 128` | [`crates/of_execution_core/src/lib.rs:9`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L9) |
| `const` | `-` | `EXECUTION_WAL_MAGIC` | `: u32 = 0x4c57_464f` | [`crates/of_execution_core/src/lib.rs:11`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L11) |
| `const` | `-` | `EXECUTION_WAL_VERSION` | `: u16 = 1` | [`crates/of_execution_core/src/lib.rs:13`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L13) |
| `const` | `-` | `EXECUTION_WAL_HEADER_LEN` | `: usize = 80` | [`crates/of_execution_core/src/lib.rs:15`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L15) |
| `const` | `-` | `EXECUTION_WAL_MAX_PAYLOAD_LEN` | `: usize = u32::MAX as usize` | [`crates/of_execution_core/src/lib.rs:17`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L17) |
| `type` | `-` | `ClientOrderId` | `= FixedAscii<40>` | [`crates/of_execution_core/src/lib.rs:116`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L116) |
| `type` | `-` | `VenueOrderId` | `= FixedAscii<48>` | [`crates/of_execution_core/src/lib.rs:118`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L118) |
| `type` | `-` | `ExecutionId` | `= FixedAscii<48>` | [`crates/of_execution_core/src/lib.rs:120`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L120) |
| `type` | `-` | `AccountId` | `= FixedAscii<32>` | [`crates/of_execution_core/src/lib.rs:122`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L122) |
| `type` | `-` | `RouteId` | `= FixedAscii<32>` | [`crates/of_execution_core/src/lib.rs:124`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L124) |
| `type` | `-` | `StrategyId` | `= FixedAscii<32>` | [`crates/of_execution_core/src/lib.rs:126`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L126) |
| `type` | `-` | `VenueId` | `= FixedAscii<16>` | [`crates/of_execution_core/src/lib.rs:128`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L128) |
| `type` | `-` | `InstrumentId` | `= FixedAscii<32>` | [`crates/of_execution_core/src/lib.rs:130`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L130) |
| `type` | `-` | `ExecutionText` | `= FixedAscii<EXECUTION_TEXT_CAP>` | [`crates/of_execution_core/src/lib.rs:132`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L132) |
| `variant` | `ExecutionCoreError` | `NonAsciiIdentifier` | `NonAsciiIdentifier` | [`crates/of_execution_core/src/lib.rs:145`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L145) |
| `variant` | `ExecutionCoreError` | `InvalidQuantity` | `InvalidQuantity` | [`crates/of_execution_core/src/lib.rs:147`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L147) |
| `variant` | `ExecutionCoreError` | `InvalidPrice` | `InvalidPrice` | [`crates/of_execution_core/src/lib.rs:149`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L149) |
| `variant` | `ExecutionCoreError` | `InvalidTransition` | `InvalidTransition` | [`crates/of_execution_core/src/lib.rs:151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L151) |
| `variant` | `WalChecksumField` | `Header` | `Header` | [`crates/of_execution_core/src/lib.rs:175`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L175) |
| `variant` | `WalChecksumField` | `Payload` | `Payload` | [`crates/of_execution_core/src/lib.rs:177`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L177) |
| `variant` | `WalRecordKind` | `CommandSubmit` | `CommandSubmit = 1` | [`crates/of_execution_core/src/lib.rs:328`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L328) |
| `variant` | `WalRecordKind` | `CommandCancel` | `CommandCancel = 2` | [`crates/of_execution_core/src/lib.rs:330`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L330) |
| `variant` | `WalRecordKind` | `CommandAmend` | `CommandAmend = 3` | [`crates/of_execution_core/src/lib.rs:332`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L332) |
| `variant` | `WalRecordKind` | `ExecutionEvent` | `ExecutionEvent = 4` | [`crates/of_execution_core/src/lib.rs:334`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L334) |
| `variant` | `WalRecordKind` | `RiskReject` | `RiskReject = 5` | [`crates/of_execution_core/src/lib.rs:336`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L336) |
| `variant` | `WalRecordKind` | `RecoveryEvent` | `RecoveryEvent = 6` | [`crates/of_execution_core/src/lib.rs:338`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L338) |
| `variant` | `WalRecordKind` | `CheckpointMarker` | `CheckpointMarker = 7` | [`crates/of_execution_core/src/lib.rs:340`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L340) |
| `variant` | `WalRecordKind` | `SegmentSeal` | `SegmentSeal = 8` | [`crates/of_execution_core/src/lib.rs:342`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L342) |
| `variant` | `WalRecordKind` | `Heartbeat` | `Heartbeat = 9` | [`crates/of_execution_core/src/lib.rs:344`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L344) |
| `variant` | `WalSyncPolicy` | `Never` | `Never` | [`crates/of_execution_core/src/lib.rs:371`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L371) |
| `variant` | `WalSyncPolicy` | `EveryRecord` | `EveryRecord` | [`crates/of_execution_core/src/lib.rs:373`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L373) |
| `variant` | `WalSyncPolicy` | `EveryNRecords` | `EveryNRecords(u32)` | [`crates/of_execution_core/src/lib.rs:375`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L375) |
| `variant` | `WalSyncPolicy` | `EveryDurationNs` | `EveryDurationNs(u64)` | [`crates/of_execution_core/src/lib.rs:377`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L377) |
| `variant` | `WalSyncPolicy` | `Manual` | `Manual` | [`crates/of_execution_core/src/lib.rs:379`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L379) |
| `variant` | `WalSyncPolicy` | `OnRiskBoundary` | `OnRiskBoundary` | [`crates/of_execution_core/src/lib.rs:381`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L381) |
| `field` | `WalRecordHeader` | `version` | `: u16` | [`crates/of_execution_core/src/lib.rs:389`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L389) |
| `field` | `WalRecordHeader` | `kind` | `: WalRecordKind` | [`crates/of_execution_core/src/lib.rs:391`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L391) |
| `field` | `WalRecordHeader` | `flags` | `: u16` | [`crates/of_execution_core/src/lib.rs:393`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L393) |
| `field` | `WalRecordHeader` | `payload_len` | `: u32` | [`crates/of_execution_core/src/lib.rs:395`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L395) |
| `field` | `WalRecordHeader` | `sequence` | `: WalSequence` | [`crates/of_execution_core/src/lib.rs:397`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L397) |
| `field` | `WalRecordHeader` | `timestamp_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:399`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L399) |
| `field` | `WalRecordHeader` | `route_hash` | `: u64` | [`crates/of_execution_core/src/lib.rs:401`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L401) |
| `field` | `WalRecordHeader` | `account_hash` | `: u64` | [`crates/of_execution_core/src/lib.rs:403`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L403) |
| `field` | `WalRecordHeader` | `symbol_hash` | `: u64` | [`crates/of_execution_core/src/lib.rs:405`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L405) |
| `field` | `WalRecordHeader` | `previous_checksum` | `: u64` | [`crates/of_execution_core/src/lib.rs:407`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L407) |
| `field` | `WalRecordHeader` | `payload_checksum` | `: u64` | [`crates/of_execution_core/src/lib.rs:409`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L409) |
| `field` | `WalRecordHeader` | `header_checksum` | `: u64` | [`crates/of_execution_core/src/lib.rs:411`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L411) |
| `field` | `WalRecordView` | `header` | `: WalRecordHeader` | [`crates/of_execution_core/src/lib.rs:517`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L517) |
| `field` | `WalRecordView` | `payload` | `: &'a [u8]` | [`crates/of_execution_core/src/lib.rs:519`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L519) |
| `field` | `WalIntegrityReport` | `records` | `: u64` | [`crates/of_execution_core/src/lib.rs:714`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L714) |
| `field` | `WalIntegrityReport` | `bytes` | `: u64` | [`crates/of_execution_core/src/lib.rs:716`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L716) |
| `field` | `WalIntegrityReport` | `first_sequence` | `: Option<WalSequence>` | [`crates/of_execution_core/src/lib.rs:718`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L718) |
| `field` | `WalIntegrityReport` | `last_sequence` | `: Option<WalSequence>` | [`crates/of_execution_core/src/lib.rs:720`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L720) |
| `field` | `WalIntegrityReport` | `checksum_failures` | `: u64` | [`crates/of_execution_core/src/lib.rs:722`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L722) |
| `field` | `WalIntegrityReport` | `sequence_failures` | `: u64` | [`crates/of_execution_core/src/lib.rs:724`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L724) |
| `field` | `WalIntegrityReport` | `truncated_tail` | `: bool` | [`crates/of_execution_core/src/lib.rs:726`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L726) |
| `field` | `WalIntegrityReport` | `valid` | `: bool` | [`crates/of_execution_core/src/lib.rs:728`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L728) |
| `field` | `ExecutionSymbol` | `venue` | `: VenueId` | [`crates/of_execution_core/src/lib.rs:892`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L892) |
| `field` | `ExecutionSymbol` | `instrument` | `: InstrumentId` | [`crates/of_execution_core/src/lib.rs:894`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L894) |
| `variant` | `OrderSide` | `Buy` | `Buy = 1` | [`crates/of_execution_core/src/lib.rs:954`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L954) |
| `variant` | `OrderSide` | `Sell` | `Sell = 2` | [`crates/of_execution_core/src/lib.rs:956`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L956) |
| `variant` | `OrderType` | `Market` | `Market = 1` | [`crates/of_execution_core/src/lib.rs:964`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L964) |
| `variant` | `OrderType` | `Limit` | `Limit = 2` | [`crates/of_execution_core/src/lib.rs:966`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L966) |
| `variant` | `OrderType` | `Stop` | `Stop = 3` | [`crates/of_execution_core/src/lib.rs:968`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L968) |
| `variant` | `OrderType` | `StopLimit` | `StopLimit = 4` | [`crates/of_execution_core/src/lib.rs:970`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L970) |
| `variant` | `TimeInForce` | `Day` | `Day = 1` | [`crates/of_execution_core/src/lib.rs:978`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L978) |
| `variant` | `TimeInForce` | `Gtc` | `Gtc = 2` | [`crates/of_execution_core/src/lib.rs:980`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L980) |
| `variant` | `TimeInForce` | `Ioc` | `Ioc = 3` | [`crates/of_execution_core/src/lib.rs:982`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L982) |
| `variant` | `TimeInForce` | `Fok` | `Fok = 4` | [`crates/of_execution_core/src/lib.rs:984`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L984) |
| `variant` | `TimeInForce` | `Gtd` | `Gtd = 5` | [`crates/of_execution_core/src/lib.rs:986`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L986) |
| `variant` | `OrderStatus` | `PendingNew` | `PendingNew = 1` | [`crates/of_execution_core/src/lib.rs:994`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L994) |
| `variant` | `OrderStatus` | `New` | `New = 2` | [`crates/of_execution_core/src/lib.rs:996`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L996) |
| `variant` | `OrderStatus` | `PartiallyFilled` | `PartiallyFilled = 3` | [`crates/of_execution_core/src/lib.rs:998`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L998) |
| `variant` | `OrderStatus` | `Filled` | `Filled = 4` | [`crates/of_execution_core/src/lib.rs:1000`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1000) |
| `variant` | `OrderStatus` | `PendingCancel` | `PendingCancel = 5` | [`crates/of_execution_core/src/lib.rs:1002`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1002) |
| `variant` | `OrderStatus` | `Cancelled` | `Cancelled = 6` | [`crates/of_execution_core/src/lib.rs:1004`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1004) |
| `variant` | `OrderStatus` | `PendingReplace` | `PendingReplace = 7` | [`crates/of_execution_core/src/lib.rs:1006`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1006) |
| `variant` | `OrderStatus` | `Replaced` | `Replaced = 8` | [`crates/of_execution_core/src/lib.rs:1008`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1008) |
| `variant` | `OrderStatus` | `Rejected` | `Rejected = 9` | [`crates/of_execution_core/src/lib.rs:1010`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1010) |
| `variant` | `OrderStatus` | `Expired` | `Expired = 10` | [`crates/of_execution_core/src/lib.rs:1012`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1012) |
| `variant` | `OrderStatus` | `Suspended` | `Suspended = 11` | [`crates/of_execution_core/src/lib.rs:1014`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1014) |
| `variant` | `OrderStatus` | `Unknown` | `Unknown = 12` | [`crates/of_execution_core/src/lib.rs:1016`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1016) |
| `variant` | `ExecutionType` | `Ack` | `Ack = 1` | [`crates/of_execution_core/src/lib.rs:1034`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1034) |
| `variant` | `ExecutionType` | `Reject` | `Reject = 2` | [`crates/of_execution_core/src/lib.rs:1036`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1036) |
| `variant` | `ExecutionType` | `Trade` | `Trade = 3` | [`crates/of_execution_core/src/lib.rs:1038`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1038) |
| `variant` | `ExecutionType` | `CancelPending` | `CancelPending = 4` | [`crates/of_execution_core/src/lib.rs:1040`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1040) |
| `variant` | `ExecutionType` | `CancelAck` | `CancelAck = 5` | [`crates/of_execution_core/src/lib.rs:1042`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1042) |
| `variant` | `ExecutionType` | `CancelReject` | `CancelReject = 6` | [`crates/of_execution_core/src/lib.rs:1044`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1044) |
| `variant` | `ExecutionType` | `ReplacePending` | `ReplacePending = 7` | [`crates/of_execution_core/src/lib.rs:1046`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1046) |
| `variant` | `ExecutionType` | `ReplaceAck` | `ReplaceAck = 8` | [`crates/of_execution_core/src/lib.rs:1048`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1048) |
| `variant` | `ExecutionType` | `ReplaceReject` | `ReplaceReject = 9` | [`crates/of_execution_core/src/lib.rs:1050`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1050) |
| `variant` | `ExecutionType` | `Expire` | `Expire = 10` | [`crates/of_execution_core/src/lib.rs:1052`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1052) |
| `variant` | `ExecutionType` | `Status` | `Status = 11` | [`crates/of_execution_core/src/lib.rs:1054`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1054) |
| `variant` | `ExecutionType` | `Restated` | `Restated = 12` | [`crates/of_execution_core/src/lib.rs:1056`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1056) |
| `variant` | `ExecutionType` | `AdapterDegraded` | `AdapterDegraded = 13` | [`crates/of_execution_core/src/lib.rs:1058`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1058) |
| `field` | `OrderRequest` | `client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1066`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1066) |
| `field` | `OrderRequest` | `account_id` | `: AccountId` | [`crates/of_execution_core/src/lib.rs:1068`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1068) |
| `field` | `OrderRequest` | `route_id` | `: RouteId` | [`crates/of_execution_core/src/lib.rs:1070`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1070) |
| `field` | `OrderRequest` | `strategy_id` | `: StrategyId` | [`crates/of_execution_core/src/lib.rs:1072`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1072) |
| `field` | `OrderRequest` | `symbol` | `: ExecutionSymbol` | [`crates/of_execution_core/src/lib.rs:1074`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1074) |
| `field` | `OrderRequest` | `side` | `: OrderSide` | [`crates/of_execution_core/src/lib.rs:1076`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1076) |
| `field` | `OrderRequest` | `order_type` | `: OrderType` | [`crates/of_execution_core/src/lib.rs:1078`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1078) |
| `field` | `OrderRequest` | `time_in_force` | `: TimeInForce` | [`crates/of_execution_core/src/lib.rs:1080`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1080) |
| `field` | `OrderRequest` | `quantity` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1082`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1082) |
| `field` | `OrderRequest` | `limit_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1084`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1084) |
| `field` | `OrderRequest` | `stop_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1086`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1086) |
| `field` | `OrderRequest` | `ts_exchange_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1088`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1088) |
| `field` | `OrderRequest` | `ts_recv_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1090`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1090) |
| `field` | `CancelRequest` | `client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1121`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1121) |
| `field` | `CancelRequest` | `orig_client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1123`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1123) |
| `field` | `CancelRequest` | `venue_order_id` | `: VenueOrderId` | [`crates/of_execution_core/src/lib.rs:1125`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1125) |
| `field` | `CancelRequest` | `account_id` | `: AccountId` | [`crates/of_execution_core/src/lib.rs:1127`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1127) |
| `field` | `CancelRequest` | `route_id` | `: RouteId` | [`crates/of_execution_core/src/lib.rs:1129`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1129) |
| `field` | `CancelRequest` | `symbol` | `: ExecutionSymbol` | [`crates/of_execution_core/src/lib.rs:1131`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1131) |
| `field` | `CancelRequest` | `ts_recv_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1133`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1133) |
| `field` | `AmendRequest` | `client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1141`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1141) |
| `field` | `AmendRequest` | `orig_client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1143`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1143) |
| `field` | `AmendRequest` | `venue_order_id` | `: VenueOrderId` | [`crates/of_execution_core/src/lib.rs:1145`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1145) |
| `field` | `AmendRequest` | `account_id` | `: AccountId` | [`crates/of_execution_core/src/lib.rs:1147`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1147) |
| `field` | `AmendRequest` | `route_id` | `: RouteId` | [`crates/of_execution_core/src/lib.rs:1149`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1149) |
| `field` | `AmendRequest` | `symbol` | `: ExecutionSymbol` | [`crates/of_execution_core/src/lib.rs:1151`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1151) |
| `field` | `AmendRequest` | `quantity` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1153`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1153) |
| `field` | `AmendRequest` | `limit_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1155`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1155) |
| `field` | `AmendRequest` | `ts_recv_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1157`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1157) |
| `field` | `ExecutionEvent` | `exec_type` | `: ExecutionType` | [`crates/of_execution_core/src/lib.rs:1165`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1165) |
| `field` | `ExecutionEvent` | `order_status` | `: OrderStatus` | [`crates/of_execution_core/src/lib.rs:1167`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1167) |
| `field` | `ExecutionEvent` | `client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1169`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1169) |
| `field` | `ExecutionEvent` | `orig_client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1171`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1171) |
| `field` | `ExecutionEvent` | `venue_order_id` | `: VenueOrderId` | [`crates/of_execution_core/src/lib.rs:1173`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1173) |
| `field` | `ExecutionEvent` | `execution_id` | `: ExecutionId` | [`crates/of_execution_core/src/lib.rs:1175`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1175) |
| `field` | `ExecutionEvent` | `account_id` | `: AccountId` | [`crates/of_execution_core/src/lib.rs:1177`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1177) |
| `field` | `ExecutionEvent` | `route_id` | `: RouteId` | [`crates/of_execution_core/src/lib.rs:1179`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1179) |
| `field` | `ExecutionEvent` | `symbol` | `: ExecutionSymbol` | [`crates/of_execution_core/src/lib.rs:1181`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1181) |
| `field` | `ExecutionEvent` | `last_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1183`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1183) |
| `field` | `ExecutionEvent` | `last_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1185`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1185) |
| `field` | `ExecutionEvent` | `cumulative_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1187`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1187) |
| `field` | `ExecutionEvent` | `leaves_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1189`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1189) |
| `field` | `ExecutionEvent` | `average_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1191`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1191) |
| `field` | `ExecutionEvent` | `ts_exchange_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1193`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1193) |
| `field` | `ExecutionEvent` | `ts_recv_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1195`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1195) |
| `field` | `ExecutionEvent` | `reason` | `: RiskRejectReason` | [`crates/of_execution_core/src/lib.rs:1197`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1197) |
| `field` | `ExecutionEvent` | `text` | `: ExecutionText` | [`crates/of_execution_core/src/lib.rs:1199`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1199) |
| `field` | `OrderState` | `client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1257`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1257) |
| `field` | `OrderState` | `last_accepted_client_order_id` | `: ClientOrderId` | [`crates/of_execution_core/src/lib.rs:1259`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1259) |
| `field` | `OrderState` | `venue_order_id` | `: VenueOrderId` | [`crates/of_execution_core/src/lib.rs:1261`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1261) |
| `field` | `OrderState` | `account_id` | `: AccountId` | [`crates/of_execution_core/src/lib.rs:1263`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1263) |
| `field` | `OrderState` | `route_id` | `: RouteId` | [`crates/of_execution_core/src/lib.rs:1265`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1265) |
| `field` | `OrderState` | `symbol` | `: ExecutionSymbol` | [`crates/of_execution_core/src/lib.rs:1267`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1267) |
| `field` | `OrderState` | `side` | `: OrderSide` | [`crates/of_execution_core/src/lib.rs:1269`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1269) |
| `field` | `OrderState` | `status` | `: OrderStatus` | [`crates/of_execution_core/src/lib.rs:1271`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1271) |
| `field` | `OrderState` | `order_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1273`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1273) |
| `field` | `OrderState` | `cumulative_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1275`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1275) |
| `field` | `OrderState` | `leaves_qty` | `: OrderQty` | [`crates/of_execution_core/src/lib.rs:1277`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1277) |
| `field` | `OrderState` | `average_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1279`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1279) |
| `field` | `OrderState` | `updated_ns` | `: u64` | [`crates/of_execution_core/src/lib.rs:1281`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1281) |
| `variant` | `RiskRejectReason` | `None` | `None = 0` | [`crates/of_execution_core/src/lib.rs:1444`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1444) |
| `variant` | `RiskRejectReason` | `KillSwitch` | `KillSwitch = 1` | [`crates/of_execution_core/src/lib.rs:1446`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1446) |
| `variant` | `RiskRejectReason` | `AccountDisabled` | `AccountDisabled = 2` | [`crates/of_execution_core/src/lib.rs:1448`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1448) |
| `variant` | `RiskRejectReason` | `RouteDisabled` | `RouteDisabled = 3` | [`crates/of_execution_core/src/lib.rs:1450`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1450) |
| `variant` | `RiskRejectReason` | `SymbolDisabled` | `SymbolDisabled = 4` | [`crates/of_execution_core/src/lib.rs:1452`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1452) |
| `variant` | `RiskRejectReason` | `MaxOrderQty` | `MaxOrderQty = 5` | [`crates/of_execution_core/src/lib.rs:1454`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1454) |
| `variant` | `RiskRejectReason` | `MaxOrderNotional` | `MaxOrderNotional = 6` | [`crates/of_execution_core/src/lib.rs:1456`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1456) |
| `variant` | `RiskRejectReason` | `MaxOpenOrders` | `MaxOpenOrders = 7` | [`crates/of_execution_core/src/lib.rs:1458`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1458) |
| `variant` | `RiskRejectReason` | `MaxOpenNotional` | `MaxOpenNotional = 8` | [`crates/of_execution_core/src/lib.rs:1460`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1460) |
| `variant` | `RiskRejectReason` | `PriceBand` | `PriceBand = 9` | [`crates/of_execution_core/src/lib.rs:1462`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1462) |
| `variant` | `RiskRejectReason` | `DuplicateClientOrderId` | `DuplicateClientOrderId = 10` | [`crates/of_execution_core/src/lib.rs:1464`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1464) |
| `variant` | `RiskRejectReason` | `UnsupportedOrderType` | `UnsupportedOrderType = 11` | [`crates/of_execution_core/src/lib.rs:1466`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1466) |
| `variant` | `RiskRejectReason` | `UnsupportedTimeInForce` | `UnsupportedTimeInForce = 12` | [`crates/of_execution_core/src/lib.rs:1468`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1468) |
| `field` | `RiskDecision` | `allowed` | `: bool` | [`crates/of_execution_core/src/lib.rs:1476`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1476) |
| `field` | `RiskDecision` | `reason` | `: RiskRejectReason` | [`crates/of_execution_core/src/lib.rs:1478`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1478) |
| `field` | `RiskDecision` | `text` | `: ExecutionText` | [`crates/of_execution_core/src/lib.rs:1480`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1480) |
| `field` | `RiskLimits` | `kill_switch` | `: bool` | [`crates/of_execution_core/src/lib.rs:1508`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1508) |
| `field` | `RiskLimits` | `max_order_qty` | `: i64` | [`crates/of_execution_core/src/lib.rs:1510`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1510) |
| `field` | `RiskLimits` | `max_order_notional` | `: i128` | [`crates/of_execution_core/src/lib.rs:1512`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1512) |
| `field` | `RiskLimits` | `max_open_orders` | `: u32` | [`crates/of_execution_core/src/lib.rs:1514`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1514) |
| `field` | `RiskLimits` | `max_open_notional` | `: i128` | [`crates/of_execution_core/src/lib.rs:1516`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1516) |
| `field` | `RiskLimits` | `price_band_ticks` | `: i64` | [`crates/of_execution_core/src/lib.rs:1518`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1518) |
| `field` | `RiskContext` | `open_orders` | `: u32` | [`crates/of_execution_core/src/lib.rs:1539`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1539) |
| `field` | `RiskContext` | `open_notional` | `: i128` | [`crates/of_execution_core/src/lib.rs:1541`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1541) |
| `field` | `RiskContext` | `reference_price` | `: OrderPrice` | [`crates/of_execution_core/src/lib.rs:1543`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1543) |
| `field` | `RiskContext` | `duplicate_client_order_id` | `: bool` | [`crates/of_execution_core/src/lib.rs:1545`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1545) |
| `field` | `RiskContext` | `account_enabled` | `: bool` | [`crates/of_execution_core/src/lib.rs:1547`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1547) |
| `field` | `RiskContext` | `route_enabled` | `: bool` | [`crates/of_execution_core/src/lib.rs:1549`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1549) |
| `field` | `RiskContext` | `symbol_enabled` | `: bool` | [`crates/of_execution_core/src/lib.rs:1551`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1551) |
| `field` | `RiskContext` | `order_type_supported` | `: bool` | [`crates/of_execution_core/src/lib.rs:1553`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1553) |
| `field` | `RiskContext` | `tif_supported` | `: bool` | [`crates/of_execution_core/src/lib.rs:1555`](https://github.com/gregorian-09/orderflow/blob/main/crates/of_execution_core/src/lib.rs#L1555) |

## Audit Requirements

The semantic review for this crate must additionally document every
public item's purpose, invariants, defaults, errors, ownership,
thread-safety, allocation/blocking behavior, persistence implications,
feature availability, introduction version, and tested usage.

- [Rust public surface audit](../rust-surface.md)
- [Rust values and layout audit](../rust-values.md)
- [Package and feature matrix](../package-matrix.md)
