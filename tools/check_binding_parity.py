#!/usr/bin/env python3
"""Validate low-level Python and Java binding parity with the C ABI manifest."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Iterable

from check_api_manifest import DEFAULT_MANIFEST, FunctionEntry, ROOT, load_manifest


DEFAULT_PYTHON_FFI = (
    ROOT / "bindings" / "python" / "orderflow" / "_generated_signatures.py"
)
DEFAULT_JAVA_NATIVE = (
    ROOT
    / "bindings"
    / "java"
    / "src"
    / "main"
    / "java"
    / "com"
    / "orderflow"
    / "bindings"
    / "OrderflowNative.java"
)
LOW_LEVEL_EXPOSURES = {
    "LowLevelGenerated",
    "HighLevelManual",
    "JsonFacade",
    "HandleFacade",
}


def manifest_symbols(entries: Iterable[FunctionEntry]) -> list[str]:
    """Return exported manifest symbols expected in low-level bindings."""

    return [
        entry.name
        for entry in entries
        if entry.exported and entry.binding_exposure in LOW_LEVEL_EXPOSURES
    ]


def parse_python_registrations(path: Path) -> tuple[set[str], set[str]]:
    """Return functions with Python ctypes argtypes and restype registrations."""

    text = path.read_text(encoding="utf-8")
    argtypes = set(re.findall(r"\blib\.(of_[a-z0-9_]+)\.argtypes\s*=", text))
    restypes = set(re.findall(r"\blib\.(of_[a-z0-9_]+)\.restype\s*=", text))
    list_assign = re.compile(
        r"(?P<var>[a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\[(?P<items>.*?)\]",
        flags=re.DOTALL,
    )
    for match in list_assign.finditer(text):
        names = set(re.findall(r"['\"](of_[a-z0-9_]+)['\"]", match.group("items")))
        if not names:
            continue
        loop = re.search(
            rf"\bfor\s+\w+\s+in\s+{re.escape(match.group('var'))}\s*:",
            text[match.end() :],
        )
        if loop is None:
            continue
        body_start = match.end() + loop.end()
        next_registration = text.find("\n        lib.", body_start)
        body_end = len(text) if next_registration == -1 else next_registration
        body = text[body_start:body_end]
        if ".argtypes" in body:
            argtypes.update(names)
        if ".restype" in body:
            restypes.update(names)
    return argtypes, restypes


def parse_java_declarations(path: Path) -> set[str]:
    """Return functions declared in the Java JNA native interface."""

    text = path.read_text(encoding="utf-8")
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.DOTALL)
    text = re.sub(r"//.*", " ", text)
    return set(
        re.findall(
            r"\b(?:int|long|void|String|Pointer)\s+(of_[a-z0-9_]+)\s*\(",
            text,
        )
    )


def validate(
    symbols: Iterable[str],
    python_ffi: Path,
    java_native: Path,
) -> list[str]:
    """Return parity errors for missing low-level binding declarations."""

    expected = set(symbols)
    py_argtypes, py_restypes = parse_python_registrations(python_ffi)
    java_symbols = parse_java_declarations(java_native)

    errors: list[str] = []
    for symbol in sorted(expected.difference(py_argtypes)):
        errors.append(f"missing Python argtypes registration: {symbol}")
    for symbol in sorted(expected.difference(py_restypes)):
        errors.append(f"missing Python restype registration: {symbol}")
    for symbol in sorted(expected.difference(java_symbols)):
        errors.append(f"missing Java JNA declaration: {symbol}")

    return errors


def main() -> int:
    """CLI entry point."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--python-ffi", type=Path, default=DEFAULT_PYTHON_FFI)
    parser.add_argument("--java-native", type=Path, default=DEFAULT_JAVA_NATIVE)
    args = parser.parse_args()

    symbols = manifest_symbols(load_manifest(args.manifest))
    errors = validate(symbols, args.python_ffi, args.java_native)
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        raise SystemExit(1)

    print(
        f"OK: {len(symbols)} manifest C ABI symbols have Python and Java "
        "low-level binding declarations"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
