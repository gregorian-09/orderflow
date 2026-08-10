#!/usr/bin/env python3
"""Generate low-level Python ctypes and Java JNA signatures.

The API manifest controls symbol membership and ordering. The validated public
C header supplies exact parameter and return types. High-level Python and Java
wrappers remain hand-written.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from check_api_manifest import (
    DEFAULT_HEADER,
    DEFAULT_MANIFEST,
    FunctionEntry,
    ROOT,
    load_manifest,
    normalize_return,
    parse_header_functions,
    validate,
)
from check_binding_parity import LOW_LEVEL_EXPOSURES


DEFAULT_PYTHON_OUTPUT = (
    ROOT / "bindings" / "python" / "orderflow" / "_generated_signatures.py"
)
DEFAULT_JAVA_OUTPUT = (
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

OPAQUE_TYPES = {
    "of_engine_t",
    "of_execution_engine_t",
    "of_execution_concurrent_engine_t",
    "of_execution_twap_algo_t",
    "of_subscription_t",
}

PYTHON_SCALARS = {
    "uint8_t": "ctypes.c_uint8",
    "uint32_t": "ctypes.c_uint32",
    "uint64_t": "ctypes.c_uint64",
    "int32_t": "ctypes.c_int32",
    "int64_t": "ctypes.c_int64",
    "double": "ctypes.c_double",
}

JAVA_SCALARS = {
    "uint8_t": "byte",
    "uint32_t": "int",
    "uint64_t": "long",
    "int32_t": "int",
    "int64_t": "long",
    "double": "double",
}

JAVA_STRUCT_TYPES = {
    "of_engine_config_t": "OfEngineConfig",
    "of_symbol_t": "OfSymbol",
    "of_trade_t": "OfTrade",
    "of_book_t": "OfBook",
    "of_external_feed_policy_t": "OfExternalFeedPolicy",
    "of_execution_route_config_t": "OfExecutionRouteConfig",
    "of_execution_order_request_t": "OfExecutionOrderRequest",
    "of_execution_cancel_request_t": "OfExecutionCancelRequest",
    "of_execution_amend_request_t": "OfExecutionAmendRequest",
    "of_execution_event_t": "OfExecutionEvent",
    "of_execution_order_state_t": "OfExecutionOrderState",
    "of_execution_health_t": "OfExecutionHealth",
    "of_execution_metrics_t": "OfExecutionMetrics",
    "of_execution_wal_integrity_report_t": "OfExecutionWalIntegrityReport",
    "of_execution_segmented_wal_integrity_report_t": (
        "OfExecutionSegmentedWalIntegrityReport"
    ),
    "of_execution_checkpoint_store_integrity_report_t": (
        "OfExecutionCheckpointStoreIntegrityReport"
    ),
    "of_execution_recovery_config_t": "OfExecutionRecoveryConfig",
    "of_execution_concurrent_config_t": "OfExecutionConcurrentConfig",
    "of_execution_command_report_t": "OfExecutionCommandReport",
    "of_execution_twap_config_t": "OfExecutionTwapConfig",
    "of_execution_algo_child_plan_t": "OfExecutionAlgoChildPlan",
    "of_execution_algo_progress_t": "OfExecutionAlgoProgress",
    "of_signal_config_parameter_t": "OfSignalConfigParameter",
    "of_signal_validation_config_t": "OfSignalValidationConfig",
    "of_signal_validation_event_t": "OfSignalValidationEvent",
}

JAVA_ARRAY_PARAMETERS = {
    ("of_execution_engine_create_multi", "routes"),
    ("of_execution_concurrent_engine_create_multi", "routes"),
    ("of_execution_submit_order", "out_events"),
    ("of_execution_cancel_order", "out_events"),
    ("of_execution_amend_order", "out_events"),
    ("of_execution_poll", "out_events"),
    ("of_execution_concurrent_try_recv_report", "out_events"),
    ("of_validate_signal_config_json", "parameters"),
    ("of_validate_signal_replay_json", "parameters"),
    ("of_validate_signal_replay_json", "events"),
}


@dataclass(frozen=True)
class CParameter:
    """One parsed C function parameter."""

    c_type: str
    name: str


@dataclass(frozen=True)
class CDeclaration:
    """One parsed C function declaration."""

    name: str
    returns: str
    parameters: tuple[CParameter, ...]
    doc_summary: str


def _parse_parameter(raw: str, function: str) -> CParameter:
    """Parse one restricted public-header parameter declaration."""

    raw = " ".join(raw.split())
    match = re.fullmatch(r"(?P<type>.+?)(?P<name>[A-Za-z_][A-Za-z0-9_]*)", raw)
    if match is None:
        raise ValueError(f"cannot parse parameter for {function}: {raw!r}")
    c_type = normalize_return(match.group("type").strip())
    return CParameter(c_type=c_type, name=match.group("name"))


def parse_header_declarations(path: Path) -> dict[str, CDeclaration]:
    """Parse exported Orderflow declarations from the restricted C header."""

    text = path.read_text(encoding="utf-8")
    pattern = re.compile(
        r"(?P<doc>/\*\*(?:(?!\*/).)*\*/)?\s*"
        r"(?P<returns>(?:const\s+)?[A-Za-z_][A-Za-z0-9_\s]*?\s*\**?)\s+"
        r"(?P<name>of_[a-z0-9_]+)\s*\((?P<parameters>.*?)\)\s*;",
        flags=re.DOTALL,
    )
    declarations: dict[str, CDeclaration] = {}
    for match in pattern.finditer(text):
        name = match.group("name")
        raw_parameters = match.group("parameters").strip()
        parameters = ()
        if raw_parameters and raw_parameters != "void":
            parameters = tuple(
                _parse_parameter(raw, name) for raw in raw_parameters.split(",")
            )
        declaration = CDeclaration(
            name=name,
            returns=normalize_return(match.group("returns")),
            parameters=parameters,
            doc_summary=_doc_summary(match.group("doc"), name),
        )
        if name in declarations:
            raise ValueError(f"duplicate header declaration: {name}")
        declarations[name] = declaration
    return declarations


def _doc_summary(raw: str | None, function: str) -> str:
    """Return a single safe Javadoc summary from a Doxygen block."""

    if raw is None:
        return f"Native declaration for {{@code {function}}}."
    lines = []
    for line in raw.removeprefix("/**").removesuffix("*/").splitlines():
        line = line.strip().removeprefix("*").strip()
        if not line:
            if lines:
                break
            continue
        lines.append(line)
    summary = " ".join(lines).replace("*/", "*&#47;")
    return summary or f"Native declaration for {{@code {function}}}."


def generated_entries(entries: Iterable[FunctionEntry]) -> list[FunctionEntry]:
    """Return exported entries represented in low-level bindings."""

    return [
        entry
        for entry in entries
        if entry.exported and entry.binding_exposure in LOW_LEVEL_EXPOSURES
    ]


def _base_pointer_type(c_type: str) -> tuple[str, int]:
    """Return a C type without qualifiers/pointers and its pointer depth."""

    pointer_depth = c_type.count("*")
    base = c_type.replace("*", " ")
    base = re.sub(r"\bconst\b", " ", base)
    return " ".join(base.split()), pointer_depth


def _python_struct_name(c_type: str) -> str:
    """Map an Orderflow C struct typedef to its ctypes class name."""

    if not c_type.startswith("of_") or not c_type.endswith("_t"):
        raise ValueError(f"unsupported Python C struct type: {c_type}")
    parts = c_type.removeprefix("of_").removesuffix("_t").split("_")
    return "Of" + "".join(part.capitalize() for part in parts)


def python_type(c_type: str, *, is_return: bool = False) -> str:
    """Map one C type to a generated ctypes expression."""

    if c_type == "void" and is_return:
        return "None"
    if c_type == "const char*":
        return "ctypes.c_char_p"
    base, pointer_depth = _base_pointer_type(c_type)
    if pointer_depth == 0:
        if base == "of_event_cb":
            return 'namespace["OfEventCallback"]'
        try:
            return PYTHON_SCALARS[base]
        except KeyError as error:
            raise ValueError(f"unsupported Python C type: {c_type}") from error
    if base == "char" and pointer_depth == 1:
        return "ctypes.c_char_p"
    if base == "char" and pointer_depth == 2:
        return "ctypes.POINTER(ctypes.c_char_p)"
    if base == "void" and pointer_depth == 1:
        return "ctypes.c_void_p"
    if base in OPAQUE_TYPES:
        if pointer_depth == 1:
            return "ctypes.c_void_p"
        if pointer_depth == 2:
            return "ctypes.POINTER(ctypes.c_void_p)"
    if base in PYTHON_SCALARS and pointer_depth == 1:
        return f"ctypes.POINTER({PYTHON_SCALARS[base]})"
    if base.startswith("of_") and base.endswith("_t") and pointer_depth == 1:
        return f'ctypes.POINTER(namespace["{_python_struct_name(base)}"])'
    raise ValueError(f"unsupported Python C pointer type: {c_type}")


def java_type(function: str, parameter: CParameter | None, c_type: str) -> str:
    """Map one C type to a JNA interface type."""

    if parameter is None:
        if c_type == "void":
            return "void"
        if c_type == "const char*":
            return "String"
        try:
            return JAVA_SCALARS[c_type]
        except KeyError as error:
            raise ValueError(f"unsupported Java return type: {c_type}") from error

    base, pointer_depth = _base_pointer_type(c_type)
    key = (function, parameter.name)
    if base == "of_event_cb" and pointer_depth == 0:
        return "OfEventCallback"
    if pointer_depth == 0:
        try:
            return JAVA_SCALARS[base]
        except KeyError as error:
            raise ValueError(f"unsupported Java C type: {c_type}") from error
    if base == "char" and pointer_depth == 1:
        return "Pointer" if function == "of_string_free" else "String"
    if base == "char" and pointer_depth == 2:
        return "PointerByReference"
    if base == "void" and pointer_depth == 1:
        return "Memory" if parameter.name == "out_buf" else "Pointer"
    if base in OPAQUE_TYPES:
        return "PointerByReference" if pointer_depth == 2 else "Pointer"
    if base == "uint32_t" and pointer_depth == 1:
        return "IntByReference"
    if base == "uint64_t" and pointer_depth == 1:
        return "LongByReference"
    if base == "of_analytics_config_t" and pointer_depth == 1:
        return "Pointer"
    if base in JAVA_STRUCT_TYPES and pointer_depth == 1:
        java_name = JAVA_STRUCT_TYPES[base]
        return f"{java_name}[]" if key in JAVA_ARRAY_PARAMETERS else java_name
    raise ValueError(f"unsupported Java C pointer type: {c_type}")


def render_python(
    entries: Iterable[FunctionEntry], declarations: dict[str, CDeclaration]
) -> str:
    """Render the private generated ctypes signature module."""

    lines = [
        '"""Generated ctypes signatures for the Orderflow C ABI.',
        "",
        "Generated by tools/generate_binding_signatures.py; do not edit.",
        '"""',
        "",
        "from __future__ import annotations",
        "",
        "import ctypes",
        "from typing import Any",
        "",
        "",
        "def _bind_symbols(lib: Any, namespace: dict[str, Any]) -> None:",
        '    """Bind every manifest-exposed native function to exact ctypes types."""',
    ]
    for entry in entries:
        declaration = declarations[entry.name]
        if declaration.parameters:
            lines.extend(["", f"    lib.{entry.name}.argtypes = ["])
            for parameter in declaration.parameters:
                lines.append(f"        {python_type(parameter.c_type)},")
            lines.append("    ]")
        else:
            lines.extend(["", f"    lib.{entry.name}.argtypes = []"])
        lines.append(
            f"    lib.{entry.name}.restype = "
            f"{python_type(declaration.returns, is_return=True)}"
        )
    return "\n".join(lines) + "\n"


def render_java(
    entries: Iterable[FunctionEntry], declarations: dict[str, CDeclaration]
) -> str:
    """Render the generated JNA native interface."""

    lines = [
        "package com.orderflow.bindings;",
        "",
        "import com.sun.jna.Library;",
        "import com.sun.jna.Memory;",
        "import com.sun.jna.Native;",
        "import com.sun.jna.Pointer;",
        "import com.sun.jna.ptr.IntByReference;",
        "import com.sun.jna.ptr.LongByReference;",
        "import com.sun.jna.ptr.PointerByReference;",
        "",
        "/**",
        " * Generated JNA mapping for the exported Orderflow C ABI.",
        " *",
        " * <p>Generated by {@code tools/generate_binding_signatures.py}; do not edit.",
        " * High-level lifecycle, ownership, and error handling remain in the manual wrappers.",
        " */",
        "public interface OrderflowNative extends Library {",
        "    /** Loads the native library from a concrete path. */",
        "    static OrderflowNative load(String path) {",
        "        return Native.load(path, OrderflowNative.class);",
        "    }",
    ]
    for entry in entries:
        declaration = declarations[entry.name]
        parameters = ", ".join(
            f"{java_type(entry.name, parameter, parameter.c_type)} {parameter.name}"
            for parameter in declaration.parameters
        )
        lines.extend(
            [
                "",
                f"    /** {declaration.doc_summary} */",
                f"    {java_type(entry.name, None, declaration.returns)} "
                f"{entry.name}({parameters});",
            ]
        )
    lines.append("}")
    return "\n".join(lines) + "\n"


def _write_or_check(path: Path, content: str, check: bool) -> bool:
    """Write generated content or report whether an existing file matches."""

    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != content:
            print(f"generated binding signature drift: {path.relative_to(ROOT)}", file=sys.stderr)
            return False
        return True
    path.write_text(content, encoding="utf-8")
    return True


def main() -> int:
    """CLI entry point."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--header", type=Path, default=DEFAULT_HEADER)
    parser.add_argument("--python-output", type=Path, default=DEFAULT_PYTHON_OUTPUT)
    parser.add_argument("--java-output", type=Path, default=DEFAULT_JAVA_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    entries = load_manifest(args.manifest)
    validate(entries, parse_header_functions(args.header))
    entries = generated_entries(entries)
    declarations = parse_header_declarations(args.header)
    missing = [entry.name for entry in entries if entry.name not in declarations]
    if missing:
        raise ValueError(f"header declarations not parsed: {', '.join(missing)}")
    for entry in entries:
        if declarations[entry.name].returns != entry.returns:
            raise ValueError(f"manifest/header return mismatch: {entry.name}")

    python_content = render_python(entries, declarations)
    java_content = render_java(entries, declarations)
    ok = _write_or_check(args.python_output, python_content, args.check)
    ok = _write_or_check(args.java_output, java_content, args.check) and ok
    if not ok:
        return 1

    action = "match" if args.check else "generated"
    print(f"OK: {len(entries)} low-level signatures {action} the manifest and header")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
