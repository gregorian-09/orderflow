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
- reusable `FixDecoder` and `FixEncoder` facades;
- session-state and sequence-tracking primitives;
- resend-range detection for inbound sequence gaps;
- sequence-reset guardrails that reject decreasing next expected sequence;
- typed session/admin builders for Logon, Heartbeat, TestRequest,
  ResendRequest, SequenceReset gap fill, and Logout;
- common tag constants and extraction helpers;
- caller-owned encode buffers that fill `BodyLength` and `CheckSum`;
- debug rendering with `|` delimiters outside the live hot path.

Not included yet:

- TCP/TLS transport;
- TCP/TLS-driven Logon/Logout/Heartbeat/TestRequest lifecycle;
- resend message replay and gap-fill generation;
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
- track inbound/outbound sequence numbers with plain integer state;
- build session/admin messages into reusable buffers without `format!`;
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

`FixSequenceTracker` adds deterministic sequence bookkeeping for future session
engines and adapters. It accepts expected inbound sequence numbers, reports
missing ranges as `FixSequenceAction::Gap`, treats lower `PossDupFlag=Y`
messages as duplicates, flags unmarked lower sequence numbers as too-low, and
assigns outbound sequence numbers monotonically.

The session/admin builders are intentionally small protocol helpers. They write
the common standard header fields and the required admin body fields into the
same strict encoder path used by `encode_message`; they do not manage sockets,
timers, resend stores, or authentication extensions.

## Roadmap

The planned next layers are:

- FIX 4.2/4.4 message builders for order entry;
- sequence persistence and resend message stores;
- resend/gap-fill policy;
- certification transcript capture;
- integration with `of_execution_adapters::fix`.
