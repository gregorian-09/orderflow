#!/usr/bin/env python3
"""Add semantic summaries to the human-facing API reference listings."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INPUT = ROOT / "docs" / "handbook" / "05-api-reference.md"
SUMMARY_ROW = re.compile(r"^\| `([^`]+)` \| `([^`]+)` \| (.+) \| \[`.*?\]\(.*?\) \| `(?:present|review)` \|$")
BULLET = re.compile(r"^(\s*- )`([^`]+)`(.*)$")
CRATE_HEADING = re.compile(r"^### `([^`]+)`$")
IMPL = re.compile(r"^\s*impl(?:<[^>]+>)?\s+([A-Za-z_][A-Za-z0-9_]*)")
FUNCTION = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
BINDING_DESCRIPTIONS = {
    "StreamKind": "Selects the event or snapshot stream delivered by the engine.",
    "Side": "Identifies bid/ask direction using the binding's stable numeric mapping.",
    "BookAction": "Selects whether a book level is inserted/replaced or deleted.",
    "DataQualityFlags": "Carries feed freshness, ordering, sequence, depth, and adapter-quality conditions.",
    "Symbol": "Identifies one venue-native instrument and its binding-side metadata.",
    "EngineConfig": "Controls engine identity, provider selection, persistence, and bounded runtime policy.",
    "ExternalFeedPolicy": "Controls stale-feed detection and external sequence enforcement.",
    "Engine": "Owns the binding lifecycle for the native market-data runtime.",
    "OrderflowEngine": "Owns the Java lifecycle for the native market-data and execution surface.",
    "EventListener": "Receives native stream events and must remain lightweight and non-reentrant.",
    "OrderflowError": "Base exception for an operation rejected by the native library.",
    "OrderflowStateError": "Exception indicating that the engine lifecycle does not allow the operation.",
    "OrderflowArgError": "Exception indicating invalid or unrepresentable caller arguments.",
}
METHOD_DESCRIPTIONS = {
    "connect": "Opens or establishes the provider/session connection.",
    "subscribe": "Registers a symbol or stream and records the requested subscription state.",
    "unsubscribe": "Removes a symbol or stream subscription and releases its active state.",
    "poll": "Processes one bounded unit of provider work and emits normalized events.",
    "on_analytics": "Feeds an analytics snapshot into the signal's decision state.",
    "quality_gate": "Evaluates whether the supplied data-quality flags permit signal output.",
    "checkpoint": "Captures the component state needed for deterministic restart.",
    "restore_checkpoint": "Validates and restores a previously captured component state.",
    "infer_features": "Evaluates the model against a validated feature vector.",
    "model_metadata": "Returns the model identity and compatibility metadata.",
    "feature_schema": "Returns the feature schema required by the model.",
    "api_version": "Returns the native ABI version used for compatibility checks.",
    "build_info": "Returns build and feature information for diagnostics.",
    "close": "Releases the owned native handle and makes further use invalid.",
    "destroy": "Releases the native or durable resource represented by the handle.",
    "start": "Starts processing after configuration and startup validation.",
    "stop": "Stops processing and begins the explicit shutdown barrier.",
    "reset": "Clears the component's documented accumulated state.",
    "snapshot": "Returns a read-only view of the component's current state.",
    "metrics": "Returns operational counters and latency/health observations.",
    "health": "Returns the component's connection, freshness, and degradation state.",
    "calibrate_confidence_bps": "Maps a raw confidence value through the configured calibration curve.",
    "external_health_tick": "Advances stale-feed supervision when the host owns the external feed loop.",
    "poll_once": "Advances one bounded host-controlled processing cycle.",
    "ingest_trade": "Validates and applies one externally supplied normalized trade.",
    "ingest_book": "Validates and applies one externally supplied normalized book update.",
    "book_snapshot": "Returns the materialized book read model for one symbol.",
    "analytics_snapshot": "Returns the current session analytics read model for one symbol.",
    "derived_analytics_snapshot": "Returns additive derived analytics for one symbol.",
    "session_candle_snapshot": "Returns the current session OHLCV-style summary.",
    "interval_candle_snapshot": "Returns the analytics summary for the requested rolling window.",
    "signal_snapshot": "Returns the current signal state, confidence, and gating result.",
    "signal_explanation": "Returns the signal's structured reason and decision context.",
    "signal_metrics": "Returns signal lifecycle, transition, and evaluation metrics.",
    "adapter_inventory": "Returns descriptors for adapters compiled into or discoverable by the native runtime.",
    "available_adapters": "Lists adapter descriptors that the binding can discover from the selected native library.",
    "signal_descriptors": "Returns discoverable signal names, versions, inputs, and configuration metadata.",
    "adapter_status": "Returns the active adapter's redacted operational status.",
    "set_external_reconnecting": "Marks an externally managed feed as reconnecting or restored.",
    "setExternalReconnecting": "Marks an externally managed feed as reconnecting or restored.",
    "adapter_inventory": "Returns descriptors for adapters compiled into or discoverable by the native runtime.",
    "adapterInventory": "Returns descriptors for adapters compiled into or discoverable by the native runtime.",
    "signal_descriptors": "Returns discoverable signal names, versions, inputs, and configuration metadata.",
    "signalDescriptors": "Returns discoverable signal names, versions, inputs, and configuration metadata.",
    "adapter_status": "Returns the active adapter's redacted operational status.",
    "adapterStatus": "Returns the active adapter's redacted operational status.",
    "signalExplanation": "Returns the signal's structured reason and decision context.",
    "apiVersion": "Returns the native ABI version used for compatibility checks.",
    "buildInfo": "Returns native build and feature information for diagnostics.",
    "pollOnce": "Advances one bounded host-controlled processing cycle.",
    "externalHealthTick": "Advances stale-feed supervision for an externally managed feed.",
    "ingestTrade": "Validates and applies one externally supplied normalized trade.",
    "ingestBook": "Validates and applies one externally supplied normalized book update.",
    "OrderflowEvent": "Represents one decoded callback event delivered from the native stream.",
    "of_engine_create": "Allocates and initializes an opaque native engine handle.",
    "of_engine_start": "Starts the native engine after configuration validation.",
    "of_engine_stop": "Stops native processing without releasing the engine handle.",
    "of_engine_destroy": "Releases the native engine handle and all owned state.",
    "of_subscribe": "Registers a symbol stream and callback subscription in the native engine.",
    "of_unsubscribe": "Removes a subscription while preserving the engine handle.",
    "of_unsubscribe_symbol": "Removes all active streams for one symbol.",
    "of_reset_symbol_session": "Clears the selected symbol's session analytics state.",
    "of_engine_poll_once": "Advances one bounded native processing cycle.",
    "of_ingest_trade": "Validates and applies one caller-supplied trade event.",
    "of_ingest_book": "Validates and applies one caller-supplied book update.",
    "of_configure_external_feed": "Configures stale-feed and sequence policy for a host-owned feed.",
    "of_external_set_reconnecting": "Marks the host-owned feed as reconnecting or restored.",
    "of_external_health_tick": "Advances stale-feed supervision for the host-owned feed.",
    "of_string_free": "Releases a string allocated by the native library.",
}


def load_summaries() -> dict[str, dict[str, str]]:
    """Load generated source summaries grouped by crate."""

    summaries: dict[str, dict[str, str]] = {}
    for page in sorted((ROOT / "docs" / "reference" / "crates").glob("*.md")):
        package = page.stem
        package_summaries: dict[str, str] = {}
        for line in page.read_text(encoding="utf-8").splitlines():
            match = SUMMARY_ROW.match(line)
            if match:
                package_summaries.setdefault(match.group(2), match.group(3))
        summaries[package] = package_summaries
    return summaries


def load_method_summaries() -> dict[str, dict[str, str]]:
    """Load method summaries with their owning impl type from Rust source."""

    result: dict[str, dict[str, str]] = {}
    for source in sorted((ROOT / "crates").glob("*/src/**/*.rs")):
        package = source.parts[source.parts.index("crates") + 1]
        package_result = result.setdefault(package, {})
        owner = ""
        docs: list[str] = []
        for line in source.read_text(encoding="utf-8").splitlines():
            impl = IMPL.match(line)
            if impl:
                owner = impl.group(1)
                docs = []
                continue
            stripped = line.strip()
            if stripped.startswith("///"):
                docs.append(stripped[3:].strip())
                continue
            function = FUNCTION.match(line)
            if function:
                summary = " ".join(docs).split(".", 1)[0].strip()
                if summary:
                    key = f"{owner}::{function.group(1)}" if owner else function.group(1)
                    package_result[key] = summary
                docs = []
                continue
            if stripped and not stripped.startswith("#"):
                docs = []
    return result


def base_name(item: str) -> str:
    """Return a lookup name from a displayed type, method, or function."""

    item = item.split(" {", 1)[0]
    item = item.split(" (", 1)[0]
    item = item.split(" ->", 1)[0]
    item = item.split("(", 1)[0]
    if "::" in item:
        item = item.rsplit("::", 1)[1]
    if "." in item:
        item = item.rsplit(".", 1)[1]
    item = item.split("<", 1)[0]
    item = item.split(":", 1)[0]
    return item.strip()


def fallback(item: str) -> str:
    """Describe a binding-only or otherwise unindexed public item."""

    name = base_name(item)
    if name in BINDING_DESCRIPTIONS:
        return BINDING_DESCRIPTIONS[name]
    if name in METHOD_DESCRIPTIONS:
        return METHOD_DESCRIPTIONS[name]
    if item.startswith("with Engine("):
        return "Scopes native engine ownership so cleanup runs when the block exits."
    if name in {"venue", "symbol", "bids", "asks", "last_sequence", "ts_exchange_ns", "ts_recv_ns", "schema_id"}:
        return "JSON field carrying the corresponding identity, materialized level, sequence, or timestamp value."
    if name in {"1", "2", "3", "4", "5", "6", "7"}:
        return "Stream identifier used when subscribing and dispatching callbacks."
    if name in {"0", "1", "2", "3", "4", "5", "6", "255"}:
        return "Native error value returned to classify the operation outcome."
    if name.startswith("of_") and name.endswith("_config_t"):
        return "C ABI configuration structure copied and validated by the native engine."
    if name in {"of_engine_t", "of_subscription_t"}:
        return "Opaque C ABI handle whose lifetime is controlled by the matching create/destroy functions."
    if name in {"of_side_t", "of_book_action_t"}:
        return "C ABI enumeration selecting the direction or book mutation represented by the event."
    if name == "of_error_t":
        return "C ABI error enumeration that classifies success, invalid input, state, I/O, backpressure, and quality outcomes."
    if name.endswith("_t") and name.startswith("of_"):
        return "C ABI data structure carrying the documented value or event fields supplied by the caller or native library."
    if name.startswith("of_") and name.endswith("_t"):
        return "C ABI data structure carrying the documented normalized value or event."
    if name.startswith("of_get_"):
        return "Serializes the requested read-only snapshot or diagnostic into the caller's buffer."
    if name.startswith("of_") and name.endswith("_free"):
        return "Releases a string or allocation returned by the native library."
    if name.startswith("of_"):
        if name in {"of_api_version", "of_build_info"}:
            return "Returns native compatibility/build metadata without mutating engine state."
        return "C ABI operation that validates its handle and arguments before changing or reading native state."
    lower = name.lower()
    if lower in {"new", "create", "with_built_ins"}:
        return "Constructs the public value or configured component."
    if lower.startswith(("get_", "get", "snapshot", "metrics", "health", "status", "build_info", "api_version")):
        return "Reads the current public state or diagnostic value without changing ownership."
    if lower.startswith(("set_", "configure", "with_", "reset", "clear", "close", "destroy", "stop")):
        return "Changes or releases the associated public lifecycle state according to its arguments."
    if lower.startswith(("start", "connect", "subscribe", "poll", "ingest", "append", "submit", "send")):
        return "Performs the corresponding bounded operation and reports validation or lifecycle failure explicitly."
    if lower.startswith(("parse", "decode", "read", "load", "replay")):
        return "Reads and validates the supplied representation, returning a typed result or diagnostic."
    if lower.startswith(("encode", "write", "save", "export")):
        return "Produces the documented representation while preserving the relevant identity and integrity contract."
    if name.isupper() or name.startswith("OF_"):
        return "Named constant used by the public compatibility contract."
    if name.endswith(("Error", "Exception")):
        return "Error type describing invalid input, lifecycle state, or failed external work."
    if name.endswith(("Flags", "Kind", "Type", "Status", "State", "Mode", "Policy")):
        return "Public classification or policy value used to make the surrounding contract explicit."
    if name.endswith(("Config", "Options", "Limits")):
        return "Configuration value controlling validation, capacity, or lifecycle behavior."
    if name.endswith(("Snapshot", "Report", "Metrics", "Result")):
        return "Read-only result describing the operation's observed state and diagnostics."
    if name.endswith(("Engine", "Adapter", "Registry", "Store", "Tracker", "Analyzer", "Planner")):
        return "Public component that owns the bounded state and operations for this subsystem."
    return "Public API item whose exact fields, values, and invariants are defined by its owning reference."


def enrich(text: str) -> str:
    """Append a semantic summary to every plain API listing bullet."""

    summaries = load_summaries()
    method_summaries = load_method_summaries()
    current_package = ""
    output: list[str] = []
    for line in text.splitlines():
        heading = CRATE_HEADING.match(line)
        if heading:
            current_package = heading.group(1)
        match = BULLET.match(line)
        if not match or line.lstrip().startswith("- ["):
            output.append(line)
            continue
        item = match.group(2)
        name = base_name(item)
        qualified = item.split("(", 1)[0].split(" ->", 1)[0].strip()
        summary = method_summaries.get(current_package, {}).get(qualified)
        if summary is None:
            summary = summaries.get(current_package, {}).get(name)
        if summary is None and current_package == "of_ffi_c":
            summary = summaries.get("of_ffi_c", {}).get(item)
        if name in {"0", "1", "2", "3", "4", "5", "6", "7", "255"}:
            if any(token in match.group(3) for token in ("BOOK", "TRADES", "ANALYTICS", "SIGNALS", "HEALTH")):
                summary = "Stream identifier used when subscribing and dispatching callbacks"
            elif any(token in match.group(3) for token in ("success", "invalid", "I/O", "auth", "backpressure", "quality", "internal")):
                summary = "Native error value returned to classify the operation outcome"
        summary = summary or fallback(item)
        if summary.endswith((".", ":", ";")):
            punctuation = ""
        else:
            punctuation = "."
        suffix = match.group(3)
        if " — " in suffix:
            suffix = suffix.split(" — ", 1)[0]
        output.append(f"{match.group(1)}`{item}`{suffix} — {summary}{punctuation}")
    return "\n".join(output).rstrip() + "\n"


def main() -> int:
    """Enrich or check the API reference file."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    source = args.input.read_text(encoding="utf-8")
    rendered = enrich(source)
    if args.check:
        if source != rendered:
            print(f"{args.input.relative_to(ROOT)} needs semantic summaries")
            return 1
        print(f"OK: {args.input.relative_to(ROOT)} has semantic summaries")
        return 0
    args.input.write_text(rendered, encoding="utf-8")
    print(f"wrote semantic summaries to {args.input.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
