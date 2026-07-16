#!/usr/bin/env python3
"""Validate the C ABI API manifest against the public header.

The manifest is intentionally small and machine-readable.  It is the first
source of truth for generated binding plumbing, export checks, and docs tables;
high-level Python and Java wrappers remain hand-written.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "bindings" / "api_manifest.toml"
DEFAULT_HEADER = ROOT / "crates" / "of_ffi_c" / "include" / "orderflow.h"
REQUIRED_FIELDS = {
    "name",
    "family",
    "returns",
    "ownership",
    "introduced",
    "binding_exposure",
    "exported",
}
VALID_EXPOSURES = {
    "NativeOnly",
    "LowLevelGenerated",
    "HighLevelManual",
    "JsonFacade",
    "HandleFacade",
    "NotYetExposed",
}


@dataclass(frozen=True)
class FunctionEntry:
    """Expanded API manifest function entry."""

    name: str
    family: str
    returns: str
    ownership: str
    introduced: str
    binding_exposure: str
    exported: bool


def normalize_return(value: str) -> str:
    """Normalize C return spelling for manifest/header comparisons."""

    value = " ".join(value.replace("\n", " ").split())
    value = value.replace(" *", "*").replace("* ", "*")
    return value


def load_manifest(path: Path) -> list[FunctionEntry]:
    """Load and expand grouped function entries from a TOML manifest."""

    with path.open("rb") as fp:
        data = tomllib.load(fp)

    defaults = data.get("defaults", {})
    entries: list[FunctionEntry] = []

    for group in data.get("function_groups", []):
        base = {**defaults, **group}
        functions = base.pop("functions", [])
        for item in functions:
            if isinstance(item, str):
                raw = {**base, "name": item}
            else:
                raw = {**base, **item}
            missing = sorted(REQUIRED_FIELDS.difference(raw))
            if missing:
                joined = ", ".join(missing)
                raise ValueError(f"{raw.get('name', '<unknown>')} missing fields: {joined}")
            if raw["binding_exposure"] not in VALID_EXPOSURES:
                raise ValueError(
                    f"{raw['name']} has invalid binding_exposure {raw['binding_exposure']!r}"
                )
            entries.append(
                FunctionEntry(
                    name=str(raw["name"]),
                    family=str(raw["family"]),
                    returns=normalize_return(str(raw["returns"])),
                    ownership=str(raw["ownership"]),
                    introduced=str(raw["introduced"]),
                    binding_exposure=str(raw["binding_exposure"]),
                    exported=bool(raw["exported"]),
                )
            )

    return entries


def parse_header_functions(path: Path) -> dict[str, str]:
    """Extract C ABI function names and return types from the public header."""

    text = path.read_text(encoding="utf-8")
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.DOTALL)
    text = re.sub(r"//.*", " ", text)
    pattern = re.compile(
        r"(?P<returns>(?:const\s+)?[A-Za-z_][A-Za-z0-9_\s\*]*?)\s+"
        r"(?P<name>of_[a-z0-9_]+)\s*\(",
        flags=re.MULTILINE,
    )
    return {
        match.group("name"): normalize_return(match.group("returns"))
        for match in pattern.finditer(text)
    }


def validate(entries: Iterable[FunctionEntry], header_functions: dict[str, str]) -> None:
    """Validate manifest entries against header declarations."""

    seen: dict[str, FunctionEntry] = {}
    errors: list[str] = []

    for entry in entries:
        if not re.fullmatch(r"of_[a-z0-9_]+", entry.name):
            errors.append(f"invalid C ABI symbol name: {entry.name}")
            continue
        previous = seen.setdefault(entry.name, entry)
        if previous is not entry:
            errors.append(f"duplicate manifest function: {entry.name}")
            continue
        header_return = header_functions.get(entry.name)
        if header_return is None:
            errors.append(f"manifest function missing from header: {entry.name}")
        elif header_return != entry.returns:
            errors.append(
                f"return mismatch for {entry.name}: manifest={entry.returns}, "
                f"header={header_return}"
            )

    manifest_names = set(seen)
    for name in sorted(set(header_functions).difference(manifest_names)):
        errors.append(f"header function missing from manifest: {name}")

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        raise SystemExit(1)


def exported_symbols(entries: Iterable[FunctionEntry]) -> list[str]:
    """Return exported manifest symbols in manifest order."""

    return [entry.name for entry in entries if entry.exported]


def main() -> int:
    """CLI entry point."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--header", type=Path, default=DEFAULT_HEADER)
    parser.add_argument(
        "--emit-exports",
        action="store_true",
        help="print exported C ABI symbol names and exit after validation",
    )
    args = parser.parse_args()

    entries = load_manifest(args.manifest)
    header_functions = parse_header_functions(args.header)
    validate(entries, header_functions)

    if args.emit_exports:
        for symbol in exported_symbols(entries):
            print(symbol)
    else:
        print(
            f"OK: {len(entries)} C ABI functions in "
            f"{args.manifest.relative_to(ROOT)} match {args.header.relative_to(ROOT)}"
        )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
