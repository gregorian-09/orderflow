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
CARGO_WORKSPACE_PATH = ROOT / "Cargo.toml"
CRATES_DIR = ROOT / "crates"
EXAMPLES_DIR = ROOT / "examples"


def read_versions() -> dict[str, str]:
    with VERSIONS_PATH.open("rb") as f:
        data = tomllib.load(f)
    bindings = data.get("bindings", {})
    if not isinstance(bindings, dict):
        raise ValueError("bindings/versions.toml must contain a [bindings] table")
    for key in ("python", "java", "rust"):
        if key not in bindings or not isinstance(bindings[key], str):
            raise ValueError(f"missing bindings.{key} version in bindings/versions.toml")
    return {
        "python": bindings["python"],
        "java": bindings["java"],
        "rust": bindings["rust"],
    }


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


def sync_rust_workspace(version: str, check: bool) -> bool:
    text = CARGO_WORKSPACE_PATH.read_text(encoding="utf-8")
    pattern = r'(?ms)(\[workspace\.package\][^\[]*?^version\s*=\s*")([^"]+)(")'
    match = re.search(pattern, text)
    if not match:
        raise ValueError("could not find [workspace.package] version in Cargo.toml")
    current = match.group(2)
    if current == version:
        return False
    if check:
        raise ValueError(
            f"rust workspace version mismatch: Cargo.toml={current}, expected={version}"
        )
    updated = re.sub(pattern, rf"\g<1>{version}\3", text, count=1)
    CARGO_WORKSPACE_PATH.write_text(updated, encoding="utf-8")
    return True


def rust_crate_versions(workspace_version: str) -> dict[str, str]:
    """Returns each crate directory's effective package version."""
    versions: dict[str, str] = {}
    for cargo_toml in sorted(CRATES_DIR.glob("*/Cargo.toml")):
        with cargo_toml.open("rb") as file:
            package = tomllib.load(file).get("package", {})
        name = package.get("name")
        declared = package.get("version")
        if not isinstance(name, str):
            raise ValueError(f"missing package.name in {cargo_toml.relative_to(ROOT)}")
        if isinstance(declared, str):
            effective = declared
        elif isinstance(declared, dict) and declared.get("workspace") is True:
            effective = workspace_version
        else:
            raise ValueError(f"missing package.version in {cargo_toml.relative_to(ROOT)}")
        versions[cargo_toml.parent.name] = effective
    return versions


def sync_rust_internal_dependency_versions(version: str, check: bool) -> int:
    crate_files = sorted(CRATES_DIR.glob("*/Cargo.toml"))
    crate_files.extend(sorted(EXAMPLES_DIR.glob("*/Cargo.toml")))
    crate_versions = rust_crate_versions(version)
    dep_pattern = re.compile(
        r'(?m)^(\s*(of_[a-z_]+)\s*=\s*\{[^\n]*\bpath\s*=\s*"\.\./(of_[^"/\n]+)"[^\n]*\bversion\s*=\s*")([^"\n]+)(")'
    )
    changed_files = 0
    mismatches: list[str] = []

    for cargo_toml in crate_files:
        text = cargo_toml.read_text(encoding="utf-8")

        def replace_dep(match: re.Match[str]) -> str:
            dep_name = match.group(2)
            target_crate = match.group(3)
            expected = crate_versions.get(target_crate)
            if expected is None:
                raise ValueError(
                    f"unknown internal path dependency target: {target_crate}"
                )
            current = match.group(4)
            if current == expected:
                return match.group(0)
            mismatches.append(
                f"{cargo_toml.relative_to(ROOT)}:{dep_name}={current}, expected={expected}"
            )
            return f"{match.group(1)}{expected}{match.group(5)}"

        updated, subs = dep_pattern.subn(replace_dep, text)
        if subs > 0 and updated != text:
            changed_files += 1
            if not check:
                cargo_toml.write_text(updated, encoding="utf-8")

    if check and mismatches:
        mismatch_preview = ", ".join(mismatches[:5])
        raise ValueError(
            "rust internal dependency version mismatch: "
            f"{mismatch_preview}"
        )
    return changed_files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="validate only; do not modify files")
    args = parser.parse_args()

    versions = read_versions()
    changed_python = sync_python(versions["python"], args.check)
    changed_java = sync_java(versions["java"], args.check)
    changed_rust_workspace = sync_rust_workspace(versions["rust"], args.check)
    changed_rust_deps = sync_rust_internal_dependency_versions(versions["rust"], args.check)

    if args.check:
        print("OK: binding versions match bindings/versions.toml")
    else:
        print(
            "Updated bindings versions:",
            f"python_changed={changed_python}",
            f"java_changed={changed_java}",
            f"rust_workspace_changed={changed_rust_workspace}",
            f"rust_crate_dependency_files_changed={changed_rust_deps}",
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover
        print(f"error: {exc}", file=sys.stderr)
        raise SystemExit(1)
