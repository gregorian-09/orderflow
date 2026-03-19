#!/usr/bin/env python3
"""Synchronize vcpkg Git registry baseline SHA in tracked docs/config files."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_REGISTRY_REPO = "https://github.com/gregorian-09/orderflow-vcpkg-registry.git"
DOC_PATH = ROOT / "docs" / "bindings" / "c.md"
CONFIG_PATH = ROOT / "packaging" / "vcpkg" / "registry" / "vcpkg-configuration.git-example.json"


def ensure_sha(value: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError(f"invalid SHA: {value}")
    return value


def resolve_registry_head_sha(repository: str) -> str:
    proc = subprocess.run(
        ["git", "ls-remote", repository, "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    )
    line = proc.stdout.strip().splitlines()
    if not line:
        raise ValueError(f"git ls-remote returned no output for {repository}")
    sha = line[0].split()[0].strip()
    return ensure_sha(sha)


def sync_git_example_config(path: pathlib.Path, repository: str, sha: str, check: bool) -> bool:
    data = json.loads(path.read_text(encoding="utf-8"))
    registries = data.get("registries", [])
    if not isinstance(registries, list):
        raise ValueError(f"{path} has invalid registries section")

    target = None
    for item in registries:
        if isinstance(item, dict) and item.get("repository") == repository:
            target = item
            break

    if target is None:
        raise ValueError(f"{path} does not contain registry repository {repository}")

    current = target.get("baseline")
    if current == sha:
        return False

    if check:
        raise ValueError(
            f"vcpkg git config baseline mismatch: current={current}, expected={sha}"
        )

    target["baseline"] = sha
    path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return True


def sync_c_binding_doc(path: pathlib.Path, repository: str, sha: str, check: bool) -> bool:
    text = path.read_text(encoding="utf-8")
    escaped_repo = re.escape(repository)
    pattern = re.compile(
        rf'("repository"\s*:\s*"{escaped_repo}",\s*\n\s*"baseline"\s*:\s*")([^"]+)(")'
    )
    match = pattern.search(text)
    if not match:
        raise ValueError(
            f"could not locate git registry repository/baseline block for {repository} in {path}"
        )

    current = match.group(2)
    if current == sha:
        return False

    if check:
        raise ValueError(f"c binding doc baseline mismatch: current={current}, expected={sha}")

    updated = pattern.sub(rf'\g<1>{sha}\3', text, count=1)
    path.write_text(updated, encoding="utf-8")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repository",
        default=DEFAULT_REGISTRY_REPO,
        help=f"Git registry repository URL (default: {DEFAULT_REGISTRY_REPO})",
    )
    parser.add_argument(
        "--sha",
        help="Explicit 40-char baseline SHA to apply/check. If omitted, fetched from git ls-remote HEAD.",
    )
    parser.add_argument("--check", action="store_true", help="validate only; do not modify files")
    args = parser.parse_args()

    sha = ensure_sha(args.sha) if args.sha else resolve_registry_head_sha(args.repository)

    changed_config = sync_git_example_config(CONFIG_PATH, args.repository, sha, args.check)
    changed_doc = sync_c_binding_doc(DOC_PATH, args.repository, sha, args.check)

    if args.check:
        print(f"OK: vcpkg git registry baseline is synchronized ({sha})")
    else:
        print(
            "Updated vcpkg baseline references:",
            f"sha={sha}",
            f"config_changed={changed_config}",
            f"doc_changed={changed_doc}",
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
