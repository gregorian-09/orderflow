#!/usr/bin/env python3
"""Generate a source-level public Rust declaration audit index."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "docs" / "reference" / "rust-surface.md"
DECLARATION = re.compile(
    r"^\s*pub\s+(?:(?:const|async|unsafe)\s+)*(struct|enum|trait|type|const|static|fn|mod)\s+([A-Za-z_][A-Za-z0-9_]*)"
)


def _looks_like_code_boundary(line: str) -> bool:
    """Return whether a line cannot belong to an attribute block."""

    stripped = line.strip()
    if not stripped or stripped.startswith("#") or stripped.startswith("///"):
        return False
    if stripped.startswith(("}", "pub ", "fn ", "struct ", "enum ", "trait ", "impl ")):
        return True
    if stripped.startswith(("use ", "mod ", "let ", "const ", "static ", "type ")):
        return True
    return stripped.endswith((";", "{"))


def _documentation_cursor(lines: list[str], declaration_index: int) -> int:
    """Find the source line from which a declaration's docs should be read.

    Rust attributes may be single-line or multiline. Walking backwards while
    treating only the first ``#[`` line as an attribute misses the continuation
    lines of ``#[allow(...)]`` and incorrectly reports documented items as gaps.
    This bounded scan skips the attribute block and stops at the first likely
    Rust code boundary, which is sufficient for a source-level audit without
    adding a Rust parser dependency.
    """

    cursor = declaration_index - 1
    attribute_start = None
    while cursor >= 0 and declaration_index - cursor <= 64:
        stripped = lines[cursor].strip()
        if stripped.startswith("///"):
            return cursor
        if stripped.startswith("#["):
            attribute_start = cursor
            break
        if _looks_like_code_boundary(lines[cursor]):
            break
        cursor -= 1

    if attribute_start is not None:
        cursor = attribute_start - 1
        while cursor >= 0:
            stripped = lines[cursor].strip()
            if stripped.startswith("///"):
                return cursor
            if _looks_like_code_boundary(lines[cursor]):
                break
            cursor -= 1
    return declaration_index - 1


@dataclass(frozen=True)
class Declaration:
    """A public Rust declaration and its source documentation status."""

    package: str
    path: str
    line: int
    kind: str
    name: str
    documented: bool
    summary: str


def packages() -> list[tuple[str, Path]]:
    """Return workspace package names and source roots."""

    completed = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(completed.stdout)
    return [
        (item["name"], Path(item["manifest_path"]).parent / "src")
        for item in metadata["packages"]
        if (Path(item["manifest_path"]).parent / "src").exists()
    ]


def collect() -> list[Declaration]:
    """Collect public declarations from all workspace source trees."""

    declarations: list[Declaration] = []
    for package, source_root in packages():
        for path in sorted(source_root.rglob("*.rs")):
            lines = path.read_text(encoding="utf-8").splitlines()
            for index, line in enumerate(lines):
                match = DECLARATION.match(line)
                if not match:
                    continue
                previous = _documentation_cursor(lines, index)
                documented = previous >= 0 and lines[previous].lstrip().startswith("///")
                summary_parts: list[str] = []
                cursor = previous
                while cursor >= 0:
                    text = lines[cursor].lstrip()
                    if text.startswith("///"):
                        summary_parts.append(text[3:].strip())
                        cursor -= 1
                        continue
                    if not text or text.startswith("#"):
                        cursor -= 1
                        continue
                    break
                summary = " ".join(reversed(summary_parts)).split(".", 1)[0].strip()
                declarations.append(
                    Declaration(
                        package=package,
                        path=path.relative_to(ROOT).as_posix(),
                        line=index + 1,
                        kind=match.group(1),
                        name=match.group(2),
                        documented=documented,
                        summary=summary or "(no source summary)",
                    )
                )
    return declarations


def render(declarations: list[Declaration]) -> str:
    """Render the declaration audit as Markdown."""

    documented = sum(item.documented for item in declarations)
    lines = [
        "# Rust Public Surface Audit",
        "",
        "> Generated by `python3 tools/generate_rust_surface.py`.",
        "> This is a source-level audit index, not a replacement for Rustdoc.",
        "",
        f"Declarations indexed: `{len(declarations)}`",
        f"Declarations with an immediately preceding `///` comment: `{documented}`",
        "",
        "The audit is intentionally conservative. It lists public item headers;",
        "fields, enum variants, associated constants, and method-level contracts",
        "are audited in the owning crate reference and generated Rustdoc.",
        "",
        "| Package | Kind | Name | Summary | Source | Documentation marker |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for item in declarations:
        marker = "present" if item.documented else "review"
        summary = item.summary.replace("|", "\\|")
        lines.append(
            f"| `{item.package}` | `{item.kind}` | `{item.name}` | "
            f"{summary} | "
            f"[`{item.path}:{item.line}`](https://github.com/gregorian-09/orderflow/blob/main/{item.path}#L{item.line}) | `{marker}` |"
        )
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    """Generate or validate the Rust declaration audit."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = render(collect())
    if args.check:
        existing = args.output.read_text(encoding="utf-8") if args.output.exists() else ""
        if existing != rendered:
            print(f"{args.output.relative_to(ROOT)} is not up to date")
            return 1
        print(f"OK: {args.output.relative_to(ROOT)} is up to date")
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    print(f"wrote {args.output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
