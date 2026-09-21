#!/usr/bin/env python3
"""Regression tests for release upload ref validation."""

from __future__ import annotations

import pathlib
import sys
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "release"))

from validate_release_ref import load_release_tag, validate_release_ref  # noqa: E402


class ValidateReleaseRefTests(unittest.TestCase):
    """Verify accepted and rejected workflow ref spellings."""

    def test_manifest_tag_is_accepted(self) -> None:
        tag = load_release_tag()
        validate_release_ref(tag, tag)
        validate_release_ref(f"refs/tags/{tag}", tag)

    def test_branch_is_rejected(self) -> None:
        tag = load_release_tag()
        with self.assertRaisesRegex(ValueError, "must run from"):
            validate_release_ref("main", tag)

    def test_different_tag_is_rejected(self) -> None:
        tag = load_release_tag()
        with self.assertRaisesRegex(ValueError, "must run from"):
            validate_release_ref("v0.0.0", tag)


if __name__ == "__main__":
    unittest.main()
