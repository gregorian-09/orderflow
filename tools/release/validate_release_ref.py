#!/usr/bin/env python3
"""Require a release workflow to run from the tag in ``release.toml``."""

from __future__ import annotations

import argparse
import os
import pathlib
import sys
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - exercised on Python < 3.11
    import tomli as tomllib


ROOT = pathlib.Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "release.toml"


def load_release_tag(path: pathlib.Path = MANIFEST_PATH) -> str:
    """Return the exact tag declared by the repository release manifest."""
    with path.open("rb") as file:
        document: dict[str, Any] = tomllib.load(file)
    release = document.get("release")
    if not isinstance(release, dict):
        raise ValueError("release.toml must contain a [release] table")
    tag = release.get("tag")
    if not isinstance(tag, str) or not tag:
        raise ValueError("release.toml release.tag must be a non-empty string")
    return tag


def validate_release_ref(ref: str, expected_tag: str | None = None) -> None:
    """Raise ``ValueError`` unless ``ref`` names the approved release tag.

    GitHub exposes a tag dispatch as either ``vX.Y.Z`` or ``refs/tags/vX.Y.Z``
    depending on the caller. Both spellings are accepted after normalization;
    branches and different tags are rejected.
    """
    normalized_ref = ref.removeprefix("refs/tags/")
    approved_tag = expected_tag if expected_tag is not None else load_release_tag()
    if normalized_ref != approved_tag:
        raise ValueError(
            f"release uploads must run from {approved_tag!r}; "
            f"received {ref!r}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--ref",
        default=os.environ.get("GITHUB_REF_NAME"),
        help="Git ref name, defaulting to GITHUB_REF_NAME",
    )
    args = parser.parse_args()
    if not args.ref:
        parser.error("--ref or GITHUB_REF_NAME is required")

    try:
        expected_tag = load_release_tag()
        validate_release_ref(args.ref, expected_tag)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"OK: release upload ref is {expected_tag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
