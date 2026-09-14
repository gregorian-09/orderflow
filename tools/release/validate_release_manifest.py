#!/usr/bin/env python3
"""Validate the repository-wide release manifest against Cargo metadata."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - exercised on Python < 3.11
    import tomli as tomllib


ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "release.toml"
BINDING_VERSIONS_PATH = ROOT / "bindings" / "versions.toml"
SEMVER_RE = re.compile(r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$")


def load_toml(path: pathlib.Path) -> dict[str, Any]:
    """Load a TOML document from ``path`` with the project's Python fallback."""
    with path.open("rb") as file:
        return tomllib.load(file)


def load_manifest(path: pathlib.Path = MANIFEST_PATH) -> dict[str, Any]:
    """Load the machine-readable release manifest."""
    return load_toml(path)


def load_binding_versions(path: pathlib.Path = BINDING_VERSIONS_PATH) -> dict[str, Any]:
    """Load binding versions used by the existing synchronization tool."""
    return load_toml(path)


def load_cargo_metadata() -> dict[str, Any]:
    """Return no-dependency Cargo metadata for the current workspace."""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def _is_semver(value: Any) -> bool:
    return isinstance(value, str) and SEMVER_RE.fullmatch(value) is not None


def _package_entries(manifest: dict[str, Any]) -> dict[str, dict[str, Any]]:
    packages = manifest.get("packages")
    if not isinstance(packages, dict):
        return {}
    return packages


def _manifest_dependency_requirement(version: str) -> set[str]:
    """Return the supported Cargo spellings for an anchored path dependency."""
    return {version, f"^{version}"}


def published_package_names(manifest: dict[str, Any]) -> list[str]:
    """Return publishable package names in manifest publication order."""
    packages = _package_entries(manifest)
    return [
        name
        for name, entry in sorted(
            (
                (name, entry)
                for name, entry in packages.items()
                if isinstance(entry, dict) and entry.get("publish") is True
            ),
            key=lambda item: item[1]["publish_order"],
        )
    ]


def validate_manifest(
    manifest: dict[str, Any],
    cargo_metadata: dict[str, Any],
    binding_versions: dict[str, Any],
) -> list[str]:
    """Return all release-manifest violations found in the supplied documents."""
    errors: list[str] = []
    release = manifest.get("release")
    packages = _package_entries(manifest)
    metadata_packages = cargo_metadata.get("packages", [])

    if not isinstance(release, dict):
        errors.append("release.toml must contain a [release] table")
        release = {}
    coordinated_version = release.get("coordinated_version")
    tag = release.get("tag")
    if not _is_semver(coordinated_version):
        errors.append("release.coordinated_version must be a semantic version")
    if not isinstance(tag, str) or tag != f"v{coordinated_version}":
        errors.append("release.tag must equal v<release.coordinated_version>")

    actual_versions = {
        package.get("name"): package.get("version")
        for package in metadata_packages
        if isinstance(package, dict) and isinstance(package.get("name"), str)
    }
    actual_names = set(actual_versions)
    manifest_names = set(packages)
    for name in sorted(actual_names - manifest_names):
        errors.append(f"workspace package {name} is missing from release.toml")
    for name in sorted(manifest_names - actual_names):
        errors.append(f"release.toml contains unknown workspace package {name}")

    publishable_orders: dict[int, str] = {}
    for name in sorted(manifest_names & actual_names):
        entry = packages[name]
        if not isinstance(entry, dict):
            errors.append(f"packages.{name} must be a table")
            continue

        expected_version = entry.get("version")
        if not _is_semver(expected_version):
            errors.append(f"packages.{name}.version must be a semantic version")
        elif actual_versions[name] != expected_version:
            errors.append(
                f"{name} version is {actual_versions[name]}, "
                f"release.toml expects {expected_version}"
            )

        family = entry.get("family")
        if not isinstance(family, str) or not family:
            errors.append(f"packages.{name}.family must be a non-empty string")

        publish = entry.get("publish")
        if not isinstance(publish, bool):
            errors.append(f"packages.{name}.publish must be true or false")
        if publish is True:
            order = entry.get("publish_order")
            if isinstance(order, bool) or not isinstance(order, int) or order < 1:
                errors.append(
                    f"packages.{name}.publish_order must be a positive integer"
                )
            elif order in publishable_orders:
                errors.append(
                    f"publication order {order} is shared by "
                    f"{publishable_orders[order]} and {name}"
                )
            else:
                publishable_orders[order] = name
        elif "publish_order" in entry:
            errors.append(f"non-publishable package {name} must not have publish_order")

    expected_orders = set(range(1, len(publishable_orders) + 1))
    actual_orders = set(publishable_orders)
    if actual_orders != expected_orders:
        missing = sorted(expected_orders - actual_orders)
        unexpected = sorted(actual_orders - expected_orders)
        if missing:
            errors.append(f"publication order has missing positions: {missing}")
        if unexpected:
            errors.append(f"publication order has unexpected positions: {unexpected}")

    manifest_paths = {
        pathlib.Path(package["manifest_path"]).resolve(): package["name"]
        for package in metadata_packages
        if isinstance(package, dict)
        and isinstance(package.get("manifest_path"), str)
        and isinstance(package.get("name"), str)
    }
    package_order = {
        name: entry.get("publish_order")
        for name, entry in packages.items()
        if isinstance(entry, dict) and entry.get("publish") is True
    }
    for package in metadata_packages:
        if not isinstance(package, dict):
            continue
        package_name = package.get("name")
        if package_name not in packages:
            continue
        for dependency in package.get("dependencies", []):
            if not isinstance(dependency, dict) or not dependency.get("path"):
                continue
            dependency_path = pathlib.Path(dependency["path"]).resolve() / "Cargo.toml"
            target_name = manifest_paths.get(dependency_path)
            if target_name is None:
                errors.append(
                    f"{package_name} has an internal dependency outside the workspace: "
                    f"{dependency_path}"
                )
                continue
            target_entry = packages.get(target_name)
            if not isinstance(target_entry, dict):
                continue
            target_version = target_entry.get("version")
            requirement = dependency.get("req")
            if requirement not in _manifest_dependency_requirement(target_version):
                errors.append(
                    f"{package_name} depends on {target_name} as {requirement!r}; "
                    f"expected an anchored {target_version} requirement"
                )
            if package_order.get(package_name) is not None and package_order.get(target_name) is not None:
                if package_order[package_name] <= package_order[target_name]:
                    errors.append(
                        f"publication order places {package_name} before its "
                        f"dependency {target_name}"
                    )

    manifest_bindings = manifest.get("bindings")
    actual_bindings = binding_versions.get("bindings")
    if not isinstance(manifest_bindings, dict):
        errors.append("release.toml must contain a [bindings] table")
    elif not isinstance(actual_bindings, dict):
        errors.append("bindings/versions.toml must contain a [bindings] table")
    else:
        for name, expected_version in manifest_bindings.items():
            if actual_bindings.get(name) != expected_version:
                errors.append(
                    f"binding {name} is {actual_bindings.get(name)}, "
                    f"release.toml expects {expected_version}"
                )
        for name in sorted(set(actual_bindings) - set(manifest_bindings)):
            errors.append(f"binding {name} is missing from release.toml")

    artifacts = manifest.get("artifacts")
    ffi_entry = packages.get("of_ffi_c")
    if not isinstance(artifacts, dict) or artifacts.get("c_sdk") != (
        ffi_entry.get("version") if isinstance(ffi_entry, dict) else None
    ):
        errors.append("artifacts.c_sdk must match packages.of_ffi_c.version")

    return errors


def _load_and_validate() -> tuple[dict[str, Any], list[str]]:
    manifest = load_manifest()
    errors = validate_manifest(manifest, load_cargo_metadata(), load_binding_versions())
    return manifest, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--print-publish-order",
        action="store_true",
        help="print one publishable crate name per line after validation",
    )
    args = parser.parse_args()

    try:
        manifest, errors = _load_and_validate()
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError, ValueError) as exc:
        print(f"error: could not validate release manifest: {exc}", file=sys.stderr)
        return 1

    if errors:
        print("Release manifest validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    if args.print_publish_order:
        for package_name in published_package_names(manifest):
            print(package_name)
    else:
        package_count = len(_package_entries(manifest))
        publish_count = len(published_package_names(manifest))
        print(
            "OK: release manifest matches "
            f"{package_count} workspace packages; {publish_count} are publishable"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
