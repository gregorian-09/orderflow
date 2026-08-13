#!/usr/bin/env python3
"""Generate navigable per-crate reference pages from the audit indexes."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "docs" / "reference" / "crates"
SURFACE_LINE = re.compile(
    r"^\| `([^`]+)` \| `([^`]+)` \| `([^`]+)` \| (.+) \| (.+) \| `(present|review)` \|$"
)
VALUE_LINE = re.compile(
    r"^\| `([^`]+)` \| `([^`]+)` \| `([^`]+)` \| `([^`]+)` \| `([^`]*)` \| (.+) \|$"
)


def packages() -> list[dict]:
    """Return workspace package metadata and manifests."""

    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    items: list[dict] = []
    for package in json.loads(result.stdout)["packages"]:
        manifest = Path(package["manifest_path"])
        with manifest.open("rb") as handle:
            cargo = tomllib.load(handle)
        items.append({"metadata": package, "manifest": manifest, "cargo": cargo})
    return items


def read_audit(path: Path, pattern: re.Pattern[str]) -> list[tuple[str, ...]]:
    """Read matching rows from a generated Markdown audit."""

    rows: list[tuple[str, ...]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            rows.append(match.groups())
    return rows


def local_dependencies(cargo: dict) -> list[str]:
    """Return local workspace dependency names."""

    names: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, value in cargo.get(section, {}).items():
            if isinstance(value, dict) and "path" in value:
                names.add(name)
    return sorted(names)


def render(package: dict, surface: list[tuple[str, ...]], values: list[tuple[str, ...]]) -> str:
    """Render one crate reference page."""

    metadata = package["metadata"]
    name = metadata["name"]
    cargo = package["cargo"]
    manifest = package["manifest"]
    features = cargo.get("features", {})
    source = f"crates/{name}/src"
    rustdoc = f"https://docs.rs/{name}/{metadata['version']}/{name}/"
    lines = [
        f"# `{name}` Reference",
        "",
        f"> Generated from `{manifest.relative_to(ROOT).as_posix()}`, `rust-surface.md`, and `rust-values.md`.",
        "",
        f"**Version:** `{metadata['version']}`  ",
        f"**Description:** {metadata.get('description') or '(not declared)'}  ",
        f"**Source:** [`{source}`](https://github.com/gregorian-09/orderflow/tree/main/{source})  ",
        f"**Generated Rustdoc:** [open `{name}` Rustdoc]({rustdoc})",
        "",
        "This page is the crate-level index. The source links and generated",
        "Rustdoc are authoritative for exact signatures, conditional compilation,",
        "multiline declarations, and implementation-specific detail.",
        "",
        "## Features",
        "",
    ]
    if features:
        lines.extend(f"- `{feature}`: {', '.join(f'`{value}`' for value in values) or 'empty feature'}" for feature, values in features.items())
    else:
        lines.append("- No crate-defined features.")
    lines.extend(["", "## Local Dependencies", ""])
    deps = local_dependencies(cargo)
    lines.extend(f"- [`{dep}`](./{dep}.md)" for dep in deps) if deps else lines.append("- No local workspace dependencies.")
    lines.extend(["", "## Public Declaration Index", "", "| Kind | Name | Summary | Source | Docs marker |", "| --- | --- | --- | --- | --- |"])
    for package_name, kind, name_value, summary, source_link, marker in surface:
        if package_name == name:
            lines.append(f"| `{kind}` | `{name_value}` | {summary} | {source_link} | `{marker}` |")
    lines.extend(["", "## Constants, Aliases, Fields, and Variants", "", "| Kind | Owner | Name | Declared type/value | Source |", "| --- | --- | --- | --- | --- |"])
    for package_name, kind, owner, item_name, declaration, source_link in values:
        if package_name == name:
            lines.append(f"| `{kind}` | `{owner}` | `{item_name}` | `{declaration}` | {source_link} |")
    lines.extend(
        [
            "",
            "## Audit Requirements",
            "",
            "The semantic review for this crate must additionally document every",
            "public item's purpose, invariants, defaults, errors, ownership,",
            "thread-safety, allocation/blocking behavior, persistence implications,",
            "feature availability, introduction version, and tested usage.",
            "",
            "- [Rust public surface audit](../rust-surface.md)",
            "- [Rust values and layout audit](../rust-values.md)",
            "- [Package and feature matrix](../package-matrix.md)",
        ]
    )
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    """Generate or validate all crate pages."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    surface = read_audit(ROOT / "docs" / "reference" / "rust-surface.md", SURFACE_LINE)
    values = read_audit(ROOT / "docs" / "reference" / "rust-values.md", VALUE_LINE)
    args.output.mkdir(parents=True, exist_ok=True)
    failures: list[str] = []
    for package in packages():
        name = package["metadata"]["name"]
        output = args.output / f"{name}.md"
        rendered = render(package, surface, values)
        if args.check:
            existing = output.read_text(encoding="utf-8") if output.exists() else ""
            if existing != rendered:
                failures.append(output.relative_to(ROOT).as_posix())
        else:
            output.write_text(rendered, encoding="utf-8")
    if args.check:
        if failures:
            print("out-of-date crate pages:")
            print("\n".join(failures))
            return 1
        print(f"OK: {len(packages())} crate pages are up to date")
    else:
        print(f"wrote {len(packages())} crate pages to {args.output.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
