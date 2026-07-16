#!/usr/bin/env python3
"""Generate the C ABI binding inventory from bindings/api_manifest.toml."""

from __future__ import annotations

import argparse
import sys
from collections import defaultdict
from pathlib import Path

from check_api_manifest import DEFAULT_MANIFEST, FunctionEntry, ROOT, load_manifest
from check_binding_parity import (
    DEFAULT_JAVA_NATIVE,
    DEFAULT_PYTHON_FFI,
    parse_java_declarations,
    parse_python_registrations,
)


DEFAULT_OUTPUT = ROOT / "docs" / "bindings" / "api-inventory.md"


def escape_cell(value: object) -> str:
    """Escape a Markdown table cell."""

    return str(value).replace("|", "\\|")


def mark(enabled: bool) -> str:
    """Return a stable Markdown marker for a compatibility cell."""

    return "yes" if enabled else "no"


def render_table(entries: list[FunctionEntry]) -> list[str]:
    """Render a Markdown table for one function family."""

    lines = [
        "| Function | Return | Ownership | Introduced | Binding Exposure |",
        "| --- | --- | --- | --- | --- |",
    ]
    for entry in entries:
        lines.append(
            "| "
            f"`{escape_cell(entry.name)}` | "
            f"`{escape_cell(entry.returns)}` | "
            f"`{escape_cell(entry.ownership)}` | "
            f"`{escape_cell(entry.introduced)}` | "
            f"`{escape_cell(entry.binding_exposure)}` |"
        )
    return lines


def render_matrix(entries: list[FunctionEntry], python_ffi: Path, java_native: Path) -> list[str]:
    """Render the per-symbol low-level binding compatibility matrix."""

    py_argtypes, py_restypes = parse_python_registrations(python_ffi)
    java_symbols = parse_java_declarations(java_native)

    lines = [
        "## Binding Compatibility Matrix",
        "",
        "The matrix reports whether each exported C ABI symbol is declared in the",
        "low-level Python ctypes layer and Java JNA layer. `yes` means the symbol",
        "has both Python `argtypes` and `restype` registrations, and a Java native",
        "interface declaration.",
        "",
        "| Function | Family | C ABI | Python ctypes | Java JNA | Exposure |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for entry in entries:
        if not entry.exported:
            continue
        python_ready = entry.name in py_argtypes and entry.name in py_restypes
        java_ready = entry.name in java_symbols
        lines.append(
            "| "
            f"`{escape_cell(entry.name)}` | "
            f"`{escape_cell(entry.family)}` | "
            "yes | "
            f"{mark(python_ready)} | "
            f"{mark(java_ready)} | "
            f"`{escape_cell(entry.binding_exposure)}` |"
        )
    lines.append("")
    return lines


def render_inventory(entries: list[FunctionEntry], python_ffi: Path, java_native: Path) -> str:
    """Render the full API inventory document."""

    groups: dict[str, list[FunctionEntry]] = defaultdict(list)
    for entry in entries:
        if entry.exported:
            groups[entry.family].append(entry)

    lines = [
        "# Binding API Inventory",
        "",
        "This file is generated from `bindings/api_manifest.toml`.",
        "Run `python3 tools/generate_api_inventory.py` after changing the C ABI manifest.",
        "",
        "The inventory tracks the stable C ABI symbols that low-level bindings and",
        "release checks use as their source of truth. Human-facing Python and Java",
        "wrappers remain documented in their binding-specific README files.",
        "",
    ]

    total = sum(len(items) for items in groups.values())
    lines.extend(
        [
            "## Summary",
            "",
            f"- Exported symbols: `{total}`",
            f"- Families: `{len(groups)}`",
            "",
        ]
    )

    lines.extend(render_matrix(entries, python_ffi, java_native))

    for family in sorted(groups):
        entries_for_family = groups[family]
        lines.extend(
            [
                f"## {family.title()}",
                "",
                f"Symbols: `{len(entries_for_family)}`",
                "",
                *render_table(entries_for_family),
                "",
            ]
        )

    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    """CLI entry point."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--python-ffi", type=Path, default=DEFAULT_PYTHON_FFI)
    parser.add_argument("--java-native", type=Path, default=DEFAULT_JAVA_NATIVE)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if the output file is not up to date",
    )
    args = parser.parse_args()

    rendered = render_inventory(
        load_manifest(args.manifest),
        args.python_ffi,
        args.java_native,
    )
    if args.check:
        existing = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
        if existing != rendered:
            print(
                f"{args.output.relative_to(ROOT)} is not up to date; "
                "run python3 tools/generate_api_inventory.py",
                file=sys.stderr,
            )
            return 1
        print(f"OK: {args.output.relative_to(ROOT)} is up to date")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    print(f"wrote {args.output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
