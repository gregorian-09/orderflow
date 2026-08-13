#!/usr/bin/env python3
"""Keep handbook public-symbol inventories aligned with generated summaries."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {
    ROOT / "docs/handbook/05i-of-execution-adapters-reference.md": "of_execution_adapters",
    ROOT / "docs/handbook/05k-of-execution-algos-reference.md": "of_execution_algos",
    ROOT / "docs/handbook/05l-of-analytics-reference.md": "of_analytics",
}
ROW = re.compile(r"^\| `(?:enum|struct|trait|type|const|fn)` \| `([^`]+)` \| (.+?) \| ")
ITEM = re.compile(r"^(\s*- )`([^`]+)`(\s*)$")


def summaries(crate: str) -> dict[str, str]:
    values: dict[str, str] = {}
    page = ROOT / "docs/reference/crates" / f"{crate}.md"
    for line in page.read_text(encoding="utf-8").splitlines():
        match = ROW.match(line)
        if match:
            values.setdefault(match.group(1), match.group(2).rstrip("."))
    return values


def enrich(path: Path, crate: str) -> str:
    index = summaries(crate)
    lines = path.read_text(encoding="utf-8").splitlines()
    in_public_inventory = False
    output: list[str] = []
    for line in lines:
        if line.startswith("## "):
            in_public_inventory = line in {"## Public API", "## Public Types", "## FIX API"}
        match = ITEM.match(line) if in_public_inventory else None
        if match and match.group(2) in index:
            line = f"{match.group(1)}`{match.group(2)}` — {index[match.group(2)]}."
        output.append(line)
    return "\n".join(output).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    failures: list[str] = []
    for path, crate in TARGETS.items():
        source = path.read_text(encoding="utf-8")
        rendered = enrich(path, crate)
        if args.check:
            if source != rendered:
                failures.append(path.relative_to(ROOT).as_posix())
        else:
            path.write_text(rendered, encoding="utf-8")
    if failures:
        print("public handbook inventories needing descriptions:")
        print("\n".join(failures))
        return 1
    print(f"{'checked' if args.check else 'enriched'} {len(TARGETS)} handbook public inventories")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
