# `of_fix` Reference

`of_fix` is the reusable FIX tag-value codec layer for Orderflow execution
adapters. It is intentionally lower level than `of_execution_adapters::fix`:
`of_fix` understands wire-format FIX frames, while the execution adapter layer
maps parsed execution reports into canonical OMS events.

## Public API Map

| Item | Kind | Purpose |
| --- | --- | --- |
| `SOH` | constant | FIX field delimiter byte `0x01` |
| `FixTag` | struct | Numeric FIX tag identifier with common tag constants |
| `FixVersion` | enum | Known FIX begin-string versions |
| `FixMsgType` | struct | Static FIX `MsgType(35)` identifier |
| `FixFieldView` | struct | Borrowed tag/value field view |
| `FixMessageView` | struct | Borrowed validated FIX frame view |
| `FixParseError` | enum | Strict parse and validation failures |
| `FixEncodeError` | enum | Encode-time validation failures |
| `FixProfileError` | enum | Dictionary/profile validation failures |
| `FixMessageRule` | struct | Required/disallowed tag rule for one message type |
| `FixDictionary` | struct | Static FIX version/profile rule set |
| `FixDecoder` | struct | Stateless decoder facade over caller-owned scratch |
| `FixEncoder` | struct | Reusable encoder with an owned output buffer |
| `FixSessionState` | enum | Session lifecycle state vocabulary |
| `FixSequenceTracker` | struct | Deterministic inbound/outbound sequence tracker |
| `FixSequenceAction` | enum | Result of observing an inbound sequence number |
| `FixSequenceError` | enum | Sequence validation/reset errors |
| `FixResendRange` | struct | Missing sequence range for resend request generation |
| `FixSessionHeader` | struct | Borrowed standard header fields for admin builders |
| `parse_message` | function | Parses and validates raw FIX bytes into caller scratch |
| `encode_message` | function | Encodes a message into a caller-owned `Vec<u8>` |
| `encode_logon` | function | Encodes Logon `<A>` |
| `encode_heartbeat` | function | Encodes Heartbeat `<0>` |
| `encode_test_request` | function | Encodes TestRequest `<1>` |
| `encode_resend_request` | function | Encodes ResendRequest `<2>` |
| `encode_sequence_reset_gap_fill` | function | Encodes SequenceReset `<4>` gap fill |
| `encode_logout` | function | Encodes Logout `<5>` |
| `checksum` | function | Computes FIX modulo-256 checksum |
| `debug_render` | function | Renders diagnostics with `|` separators |

## Design Boundary

`of_fix` does not implement a broker-certified session engine yet. It provides
the allocation-light codec foundation required by that future session layer.

Included:

- borrowed parsing from `&[u8]`;
- caller-provided `&mut [FixFieldView]` scratch;
- strict `BeginString(8)`, `BodyLength(9)`, and `CheckSum(10)` validation;
- common execution/session tag constants;
- typed version and known message-type helpers;
- direct extraction helpers for `MsgType(35)`, `MsgSeqNum(34)`, and
  `PossDupFlag(43)`;
- static dictionary/profile validation for required and disallowed tags;
- reusable encoder/decoder facades for components that prefer explicit codec
  objects;
- session-state and sequence-tracking primitives for future transports and
  adapters;
- typed session/admin message builders for common session flow;
- encoding into caller-owned buffers with computed `BodyLength` and `CheckSum`;
- diagnostic rendering outside the hot path.

Not included:

- TCP/TLS transport;
- TCP/TLS-driven Logon/Logout/Heartbeat/TestRequest lifecycle;
- resend message replay and gap-fill generation;
- persistent session store;
- repeating group dictionaries;
- certification harness;
- OMS execution-event mapping.

Those layers should build on top of `of_fix` rather than duplicating wire codec
logic in each adapter.

## Low-Latency Rules

The codec follows the production FIX plan in `new_features.md`:

- parse from bytes, not strings;
- borrow field values from the raw frame;
- avoid `HashMap<Tag, String>` as the primary representation;
- avoid allocation during parse after the caller supplies scratch;
- validate body length and checksum directly over the raw buffer;
- keep dictionary rules as static borrowed slices;
- track sequence state with plain integer counters;
- encode admin/session messages with preallocated caller buffers;
- keep debug formatting opt-in;
- encode into reusable caller-owned buffers.

## Decode Flow

```mermaid
flowchart LR
  Raw[raw FIX bytes] --> Scan[field scan]
  Scan --> Validate[BodyLength + CheckSum]
  Validate --> Scratch[caller scratch: FixFieldView]
  Scratch --> View[FixMessageView]
  View --> Extract[typed tag extraction]
```

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
let message = parse_message(&raw, &mut scratch)?;

assert_eq!(message.msg_type(), Some(b"0".as_slice()));
assert_eq!(message.msg_seq_num(), Some(1));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Encode Example

```rust
use of_fix::{encode_message, parse_message, FixFieldView, FixTag};

let mut raw = Vec::with_capacity(256);
encode_message(
    &mut raw,
    b"FIX.4.4",
    b"D",
    &[
        (FixTag::MSG_SEQ_NUM, b"7".as_slice()),
        (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
        (FixTag::SYMBOL, b"BTCUSDT".as_slice()),
    ],
)?;

let mut scratch = [FixFieldView::empty(); 16];
let message = parse_message(&raw, &mut scratch)?;
assert_eq!(message.get(FixTag::CL_ORD_ID), Some(b"ORD-1".as_slice()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Typed Encoder/Decoder Example

```rust
use of_fix::{FixDecoder, FixEncoder, FixFieldView, FixMsgType, FixTag, FixVersion};

let mut encoder = FixEncoder::with_capacity(256);
let raw = encoder.encode_typed(
    FixVersion::Fix44,
    FixMsgType::NEW_ORDER_SINGLE,
    &[
        (FixTag::MSG_SEQ_NUM, b"7".as_slice()),
        (FixTag::CL_ORD_ID, b"ORD-1".as_slice()),
    ],
)?;

let decoder = FixDecoder::new();
let mut scratch = [FixFieldView::empty(); 16];
let message = decoder.parse(raw, &mut scratch)?;
assert_eq!(message.version(), Some(FixVersion::Fix44));
assert_eq!(message.typed_msg_type(), Some(FixMsgType::NEW_ORDER_SINGLE));
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Profile Validation Example

`FixDictionary` performs an optional validation pass after raw wire validation.
It is deliberately static and borrowed: rules can be defined once, shared across
sessions, and evaluated without per-message heap allocation.

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
let message = parse_message(&raw, &mut scratch)?;
dictionary.validate(&message)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Profile validation checks:

- the parsed `BeginString(8)` matches the dictionary `FixVersion`;
- a rule exists for raw `MsgType(35)`;
- all configured required tags are present;
- all configured disallowed tags are absent.

It does not replace session validation, resend handling, venue certification, or
counterparty-specific business validation.

## Sequence Tracking Example

`FixSequenceTracker` is the first session primitive. It does not open sockets,
send Logon, persist sequence numbers, or replay resend ranges. It gives future
session engines one deterministic place for inbound/outbound sequence rules.

```rust
use of_fix::{FixSequenceAction, FixSequenceTracker};

let mut tracker = FixSequenceTracker::new();

assert_eq!(
    tracker.observe_inbound(1, false)?,
    FixSequenceAction::Accept { seq_no: 1 },
);

let action = tracker.observe_inbound(4, false)?;
assert!(matches!(
    action,
    FixSequenceAction::Gap { expected: 2, received: 4, .. },
));
# Ok::<(), of_fix::FixSequenceError>(())
```

Sequence behavior:

- accepted expected inbound messages advance `next_inbound`;
- higher inbound messages return `FixSequenceAction::Gap` with a resend range
  and do not advance state;
- lower inbound messages with `PossDupFlag(43)=Y` return `Duplicate`;
- lower inbound messages without `PossDupFlag(43)=Y` return `TooLow`;
- outbound sequence numbers are assigned monotonically;
- `apply_sequence_reset` can advance, but not decrease, the next expected
  inbound sequence.

## Session Admin Builder Example

The admin builders write standard header fields and common session body fields
through the same strict encoder path as `encode_message`.

```rust
use of_fix::{
    encode_logon, encode_resend_request, FixResendRange, FixSessionHeader, FixVersion,
};

let header = FixSessionHeader::new(
    b"CLIENT",
    b"BROKER",
    1,
    b"20260717-12:00:00.000",
);

let mut out = Vec::with_capacity(256);
encode_logon(&mut out, FixVersion::Fix44, header, 30, true)?;

let resend_header = FixSessionHeader::new(
    b"CLIENT",
    b"BROKER",
    2,
    b"20260717-12:00:01.000",
);
encode_resend_request(
    &mut out,
    FixVersion::Fix44,
    resend_header,
    FixResendRange { begin_seq_no: 4, end_seq_no: 9 },
)?;
# Ok::<(), of_fix::FixEncodeError>(())
```

Builder boundary:

- Logon writes `EncryptMethod(98)=0`, `HeartBtInt(108)`, and optional
  `ResetSeqNumFlag(141)=Y`;
- Heartbeat optionally writes `TestReqID(112)`;
- TestRequest writes `TestReqID(112)`;
- ResendRequest writes `BeginSeqNo(7)` and `EndSeqNo(16)`;
- SequenceReset gap fill writes `GapFillFlag(123)=Y` and `NewSeqNo(36)`;
- Logout optionally writes `Text(58)`.

The builders do not authenticate, open connections, run timers, persist sent
messages, or decide whether an application message may be resent.

## Validation Semantics

`parse_message` rejects:

- empty frames;
- malformed fields;
- non-numeric tags;
- missing required tags `8`, `9`, or `10`;
- too-small scratch buffers;
- invalid or mismatched `BodyLength(9)`;
- invalid or mismatched `CheckSum(10)`.

The strict default is intentional. Venue-specific relaxed behavior belongs in a
future profile layer so operators can see exactly what a counterparty requires.

## Encoder Semantics

`encode_message` owns tags `8`, `9`, `35`, and `10`.

The caller provides:

- begin string;
- message type;
- ordered body fields.

The encoder:

- clears the output buffer;
- writes `BeginString`;
- reserves and patches `BodyLength`;
- writes `MsgType` and caller fields;
- computes and appends `CheckSum`.

It rejects SOH bytes inside values and rejects caller-provided reserved tags.

## Roadmap

The next layers should remain additive:

- typed builders for NewOrderSingle, Cancel, Replace, and session admin
  messages;
- sequence persistence and resend message stores;
- resend/gap-fill policy and message generation;
- transcript capture;
- integration into `of_execution_adapters::fix`.
