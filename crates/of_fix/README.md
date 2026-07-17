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
- common tag constants and extraction helpers;
- caller-owned encode buffers that fill `BodyLength` and `CheckSum`;
- debug rendering with `|` delimiters outside the live hot path.

Not included yet:

- TCP/TLS transport;
- Logon/Logout/Heartbeat/TestRequest session lifecycle;
- resend request and sequence-reset handling;
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

## Roadmap

The planned next layers are:

- FIX dictionaries and profile validation;
- FIX 4.2/4.4 message builders for order entry;
- session state and sequence persistence;
- resend/gap-fill policy;
- certification transcript capture;
- integration with `of_execution_adapters::fix`.
