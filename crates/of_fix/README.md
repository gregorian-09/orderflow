# `of_fix`

[![Crates.io](https://img.shields.io/crates/v/of_fix.svg)](https://crates.io/crates/of_fix)
[![Docs.rs](https://docs.rs/of_fix/badge.svg)](https://docs.rs/of_fix)
[![CI](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gregorian-09/orderflow/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](https://opensource.org/license/mit)

`of_fix` is the reusable FIX tag-value codec foundation for Orderflow execution
adapters. It is intentionally separate from `of_execution_adapters` so FIX
wire handling, validation, profiles, and later certification tooling can evolve
without coupling the low-level protocol layer to one broker adapter.

## First Release: 0.1.0

`of_fix` starts at `0.1.0` in the broader Orderflow `0.4.0` development line.
The first slice is a codec foundation, not a complete FIX session engine.

Included now:

- borrowed tag-value field views;
- caller-owned parse scratch buffers;
- FIX `BodyLength(9)` validation;
- FIX `CheckSum(10)` validation;
- typed FIX versions and common `MsgType(35)` constants;
- static dictionary/profile validation for required and disallowed tags;
- borrowed Session Reject and BusinessMessageReject parsers for counterparty
  diagnostics;
- reusable `FixDecoder` and `FixEncoder` facades;
- session-state and sequence-tracking primitives;
- resend-range detection for inbound sequence gaps;
- sequence-reset guardrails that reject decreasing next expected sequence;
- borrowed session identity and sequence snapshot primitives for persistence;
- bounded in-memory resend-store primitives for replay/gap-fill planning;
- possible-duplicate replay encoding with `PossDupFlag(43)` and
  `OrigSendingTime(122)`;
- typed session/admin builders for Logon, Heartbeat, TestRequest,
  ResendRequest, SequenceReset gap fill, and Logout;
- typed order-entry builders for NewOrderSingle, OrderCancelRequest,
  OrderCancelReplaceRequest, OrderStatusRequest, OrderMassCancelRequest, and
  OrderMassStatusRequest, including optional `Account(1)` on single-order
  entry/cancel/replace messages;
- common tag constants and extraction helpers;
- caller-owned encode buffers that fill `BodyLength` and `CheckSum`;
- debug rendering with `|` delimiters outside the live hot path.

Not included yet:

- TCP/TLS transport;
- TCP/TLS-driven Logon/Logout/Heartbeat/TestRequest lifecycle;
- durable resend message persistence;
- automatic resend response transmission;
- persistent session store;
- repeating group dictionaries;
- venue certification harness;
- OMS execution mapping.

## Low-Latency Design

The codec is designed for execution hot paths:

- parse from `&[u8]`, not `String`;
- expose borrowed value slices instead of allocating field strings;
- use caller-provided `&mut [FixFieldView]` scratch for parse output;
- avoid `HashMap<Tag, String>` as the primary representation;
- validate `BodyLength` and `CheckSum` directly over the raw byte buffer;
- keep profile rules as static borrowed slices;
- expose reject diagnostics as borrowed views;
- track inbound/outbound sequence numbers with plain integer state;
- snapshot sequence counters without tying the codec to a storage backend;
- retain outbound resend frames behind explicit message/byte bounds;
- plan replay versus gap-fill actions into caller-owned buffers;
- rewrite replayed frames without a generic object model;
- build session/admin messages into reusable buffers without `format!`;
- build order-entry messages from borrowed identifiers, symbols, quantities,
  prices, and timestamps;
- encode into caller-owned `Vec<u8>` buffers;
- keep debug rendering opt-in and outside hot paths.

## Decode Example

```rust
use of_fix::{encode_message, parse_message, FixFieldView, FixTag};

let mut raw = Vec::new();
encode_message(
    &mut raw,
    b"FIX.4.4",
    b"0",
    &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
)?;

let mut scratch = [FixFieldView::empty(); 8];
let msg = parse_message(&raw, &mut scratch)?;

assert_eq!(msg.get(FixTag::MSG_TYPE), Some(b"0".as_slice()));
assert_eq!(msg.msg_type(), Some(b"0".as_slice()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Encode Example

```rust
use of_fix::{encode_message, FixTag};

let mut out = Vec::new();
encode_message(
    &mut out,
    b"FIX.4.4",
    b"D",
    &[(FixTag::MSG_SEQ_NUM, b"1".as_slice())],
)?;

assert!(out.starts_with(b"8=FIX.4.4\x019="));
# Ok::<(), of_fix::FixEncodeError>(())
```

## Profile Validation Example

```rust
use of_fix::{
    encode_message, parse_message, FixDictionary, FixFieldView, FixMessageRule,
    FixMsgType, FixTag, FixVersion,
};

static REQUIRED: &[FixTag] = &[FixTag::CL_ORD_ID, FixTag::SYMBOL, FixTag::SIDE];
static RULES: &[FixMessageRule<'static>] = &[FixMessageRule::new(
    FixMsgType::NEW_ORDER_SINGLE,
    REQUIRED,
    &[],
)];

let dictionary = FixDictionary::new(FixVersion::Fix44, RULES);

let mut raw = Vec::new();
encode_message(
    &mut raw,
    b"FIX.4.4",
    b"D",
    &[
        (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
        (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
        (FixTag::SIDE, b"1".as_slice()),
    ],
)?;

let mut scratch = [FixFieldView::empty(); 16];
let msg = parse_message(&raw, &mut scratch)?;
dictionary.validate(&msg)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Reject Parsing Example

```rust
use of_fix::{encode_message, parse_message, parse_session_reject, FixFieldView, FixTag};

let mut raw = Vec::new();
encode_message(
    &mut raw,
    b"FIX.4.4",
    b"3",
    &[
        (FixTag::REF_SEQ_NUM, b"12".as_slice()),
        (FixTag::REF_TAG_ID, b"55".as_slice()),
        (FixTag::REF_MSG_TYPE, b"D".as_slice()),
        (FixTag::SESSION_REJECT_REASON, b"1".as_slice()),
        (FixTag::TEXT, b"missing symbol".as_slice()),
    ],
)?;

let mut scratch = [FixFieldView::empty(); 16];
let msg = parse_message(&raw, &mut scratch)?;
let reject = parse_session_reject(&msg)?;

assert_eq!(reject.ref_seq_num(), 12);
assert_eq!(reject.ref_tag_id(), Some(FixTag::SYMBOL));
assert_eq!(reject.ref_msg_type(), Some(b"D".as_slice()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Sequence Tracking Example

```rust
use of_fix::{FixSequenceAction, FixSequenceTracker};

let mut tracker = FixSequenceTracker::new();

assert_eq!(
    tracker.observe_inbound(1, false)?,
    FixSequenceAction::Accept { seq_no: 1 },
);

assert!(matches!(
    tracker.observe_inbound(4, false)?,
    FixSequenceAction::Gap { expected: 2, received: 4, .. },
));
# Ok::<(), of_fix::FixSequenceError>(())
```

## Sequence Snapshot Example

```rust
use of_fix::{FixSequenceTracker, FixSessionId, FixVersion};

let session = FixSessionId::new(FixVersion::Fix44, b"CLIENT", b"BROKER")?;
let tracker = FixSequenceTracker::from_next(12, 34);

let snapshot = tracker.snapshot(session, b"20260717")?;
let restored = FixSequenceTracker::from_snapshot(&snapshot);

assert_eq!(restored.next_inbound(), 12);
assert_eq!(restored.next_outbound(), 34);
# Ok::<(), of_fix::FixEncodeError>(())
```

## Session Admin Builder Example

```rust
use of_fix::{encode_logon, FixSessionHeader, FixVersion};

let header = FixSessionHeader::new(
    b"CLIENT",
    b"BROKER",
    1,
    b"20260717-12:00:00.000",
);

let mut out = Vec::with_capacity(256);
encode_logon(&mut out, FixVersion::Fix44, header, 30, true)?;

assert!(out.starts_with(b"8=FIX.4.4\x019="));
# Ok::<(), of_fix::FixEncodeError>(())
```

## Order Builder Example

```rust
use of_fix::{
    encode_new_order_single, FixNewOrderSingle, FixOrdType, FixOrderSide,
    FixSessionHeader, FixTimeInForce, FixVersion,
};

let header = FixSessionHeader::new(
    b"CLIENT",
    b"BROKER",
    7,
    b"20260717-12:00:05.000",
);

let order = FixNewOrderSingle::new(
    b"ORD-1",
    b"BTCUSDT",
    FixOrderSide::Buy,
    b"20260717-12:00:05.000",
    b"1.25",
    FixOrdType::Limit,
)
.with_price(b"65000.5")
.with_time_in_force(FixTimeInForce::Day);

let mut out = Vec::with_capacity(512);
encode_new_order_single(&mut out, FixVersion::Fix44, header, order)?;
# Ok::<(), of_fix::FixEncodeError>(())
```

## Resend Planning Example

```rust
use of_fix::{
    FixResendAction, FixResendRange, FixResendStore, FixResendStoreConfig,
    FixSentMessageKind,
};

let mut store = FixResendStore::new(FixResendStoreConfig::new(128, 64 * 1024));
store.record_sent(1, FixSentMessageKind::Application, b"raw-new-order")?;
store.record_sent(2, FixSentMessageKind::Administrative, b"raw-heartbeat")?;
store.record_sent(3, FixSentMessageKind::Application, b"raw-cancel")?;

let mut actions = Vec::new();
let summary = store.plan_resend_range(
    FixResendRange {
        begin_seq_no: 1,
        end_seq_no: 3,
    },
    &mut actions,
);

assert_eq!(summary.replay_messages(), 2);
assert_eq!(
    actions,
    vec![
        FixResendAction::Replay {
            seq_no: 1,
            raw: b"raw-new-order".as_slice(),
        },
        FixResendAction::GapFill {
            begin_seq_no: 2,
            end_seq_no: 2,
        },
        FixResendAction::Replay {
            seq_no: 3,
            raw: b"raw-cancel".as_slice(),
        },
    ]
);
# Ok::<(), of_fix::FixResendStoreError>(())
```

## Possible-Duplicate Replay Example

```rust
use of_fix::{
    encode_heartbeat, encode_poss_dup_replay, parse_message, FixFieldView,
    FixSessionHeader, FixTag, FixVersion,
};

let header = FixSessionHeader::new(
    b"CLIENT",
    b"BROKER",
    42,
    b"20260717-12:00:00.000",
);

let mut original = Vec::new();
encode_heartbeat(&mut original, FixVersion::Fix44, header, None)?;

let mut scratch = [FixFieldView::empty(); 16];
let source = parse_message(&original, &mut scratch)?;

let mut replay = Vec::new();
encode_poss_dup_replay(&mut replay, &source, b"20260717-12:00:05.000")?;

let mut replay_scratch = [FixFieldView::empty(); 20];
let replayed = parse_message(&replay, &mut replay_scratch)?;

assert_eq!(replayed.msg_seq_num(), Some(42));
assert_eq!(replayed.get(FixTag::POSS_DUP_FLAG), Some(b"Y".as_slice()));
assert_eq!(
    replayed.get(FixTag::ORIG_SENDING_TIME),
    Some(b"20260717-12:00:00.000".as_slice())
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validation Semantics

`parse_message` rejects frames when:

- required tags `8`, `9`, or `10` are missing;
- fields are malformed;
- tags are non-numeric;
- the provided scratch buffer is too small;
- `BodyLength(9)` does not match the byte count from the body start through
  the delimiter before tag `10`;
- `CheckSum(10)` is not exactly three digits or does not equal the modulo-256
  byte sum before the checksum field.

This is deliberately strict for production execution paths. If a counterparty
requires a relaxed policy, that should live in a profile layer above this codec.

`FixDictionary` adds an optional second validation phase after the wire frame is
valid. It checks the parsed `BeginString(8)` against a configured
`FixVersion`, locates a `FixMessageRule` by raw `MsgType(35)`, then verifies
required and disallowed tags using borrowed field views. It does not allocate,
does not perform transport/session validation, and does not imply venue
certification.

`parse_session_reject` and `parse_business_message_reject` expose borrowed
views over Reject `<3>` and BusinessMessageReject `<j>` diagnostics. They parse
numeric reason fields and referenced sequence/tag values, but they do not
allocate strings, classify venue severity, or decide whether trading should
stop. Session engines and adapter profiles should turn these diagnostics into
health, metrics, and fail-closed policies.

`FixSequenceTracker` adds deterministic sequence bookkeeping for future session
engines and adapters. It accepts expected inbound sequence numbers, reports
missing ranges as `FixSequenceAction::Gap`, treats lower `PossDupFlag=Y`
messages as duplicates, flags unmarked lower sequence numbers as too-low, and
assigns outbound sequence numbers monotonically.

`FixSessionId` and `FixSequenceSnapshot` provide borrowed, storage-neutral
state snapshots. They do not write files or choose durability policy; WAL,
checkpoint, or database layers can serialize the session id, trading day, and
next inbound/outbound counters according to their own latency and durability
requirements.

`FixResendStore` retains original outbound FIX frames behind explicit message
and byte budgets. It produces caller-owned resend plans with replay actions for
retained application/reject messages and gap-fill actions for administrative,
missing, aged, or evicted ranges. It is not durable storage and does not
transmit responses; a session engine remains responsible for encoding
SequenceReset gap fills, setting `PossDupFlag(43)`/`OrigSendingTime(122)` on
replayed messages where required by profile, and enforcing counterparty policy.

`encode_poss_dup_replay` handles the common replay rewrite step for retained
messages. It takes a validated borrowed source message, preserves its original
sequence number and application fields, writes `PossDupFlag(43)=Y`, replaces
`SendingTime(52)` with the current send time, writes `OrigSendingTime(122)`,
and recomputes `BodyLength(9)` and `CheckSum(10)`. It does not decide which
messages are replayable or send them on a socket.

The session/admin builders are intentionally small protocol helpers. They write
the common standard header fields and the required admin body fields into the
same strict encoder path used by `encode_message`; they do not manage sockets,
timers, durable resend stores, or authentication extensions.

The order-entry builders provide common message shapes for NewOrderSingle,
OrderCancelRequest, OrderCancelReplaceRequest, OrderStatusRequest,
OrderMassCancelRequest, and OrderMassStatusRequest. They do not decide whether
a limit price is required, whether a field can be changed on replace, how
quantities are rounded, whether a mass-cancel or mass-status scope is enabled
at a venue, or which party groups, clearing instructions, or custom tags a
venue requires. Those checks belong in profile/certification layers above the
codec.

## Roadmap

The planned next layers are:

- FIX 4.2/4.4 message builders for order entry;
- durable session sequence persistence;
- venue/profile-specific resend suppression policy;
- order mass cancel/status response parsers;
- certification transcript capture;
- integration with `of_execution_adapters::fix`.
