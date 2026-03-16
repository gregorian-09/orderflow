#!/usr/bin/env python3
"""Synchronize binding package versions from bindings/versions.toml."""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[2]
VERSIONS_PATH = ROOT / "bindings" / "versions.toml"
PYPROJECT_PATH = ROOT / "bindings" / "python" / "pyproject.toml"
JAVA_POM_PATH = ROOT / "bindings" / "java" / "pom.xml"


def read_versions() -> dict[str, str]:
    with VERSIONS_PATH.open("rb") as f:
        data = tomllib.load(f)
    bindings = data.get("bindings", {})
    if not isinstance(bindings, dict):
        raise ValueError("bindings/versions.toml must contain a [bindings] table")
    for key in ("python", "java"):
        if key not in bindings or not isinstance(bindings[key], str):
            raise ValueError(f"missing bindings.{key} version in bindings/versions.toml")
    return {"python": bindings["python"], "java": bindings["java"]}


def sync_python(version: str, check: bool) -> bool:
    text = PYPROJECT_PATH.read_text(encoding="utf-8")
    current_match = re.search(r'(?m)^version = "([^"]+)"$', text)
    if not current_match:
        raise ValueError("could not find [project] version in pyproject.toml")
    current = current_match.group(1)
    if current == version:
        return False
    if check:
        raise ValueError(f"python version mismatch: pyproject={current}, expected={version}")
    updated = re.sub(r'(?m)^version = "([^"]+)"$', f'version = "{version}"', text, count=1)
    PYPROJECT_PATH.write_text(updated, encoding="utf-8")
    return True


def sync_java(version: str, check: bool) -> bool:
    text = JAVA_POM_PATH.read_text(encoding="utf-8")
    pattern = (
        r"(<artifactId>orderflow-java-binding</artifactId>\s*"
        r"<version>)([^<]+)(</version>)"
    )
    match = re.search(pattern, text)
    if not match:
        raise ValueError("could not locate orderflow-java-binding version in pom.xml")
    current = match.group(2).strip()
    if current == version:
        return False
    if check:
        raise ValueError(f"java version mismatch: pom={current}, expected={version}")
    updated = re.sub(pattern, rf"\g<1>{version}\3", text, count=1)
    JAVA_POM_PATH.write_text(updated, encoding="utf-8")
    return True


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate only; do not modify files")
    args = parser.parse_args()

    versions = read_versions()
    changed_python = sync_python(versions["python"], args.check)
    changed_java = sync_java(versions["java"], args.check)

    if args.check:
        print("OK: binding versions match bindings/versions.toml")
    else:
        print(
            "Updated bindings versions:",
            f"python_changed={changed_python}",
            f"java_changed={changed_java}",
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
