#!/usr/bin/env python3
"""Unit tests for mixed-version Rust workspace synchronization."""

from __future__ import annotations

import unittest

import sync_binding_versions


class SyncBindingVersionsTests(unittest.TestCase):
    """Covers established and independently versioned crate families."""

    def test_effective_crate_versions_follow_each_manifest(self) -> None:
        """Workspace inheritance and explicit crate versions remain distinct."""
        versions = sync_binding_versions.rust_crate_versions("0.4.0")
        self.assertEqual(versions["of_core"], "0.4.0")
        self.assertEqual(versions["of_ffi_c"], "0.4.0")
        self.assertEqual(versions["of_analytics"], "0.1.0")
        self.assertEqual(versions["of_execution_algos"], "0.1.0")
        self.assertEqual(versions["of_fix"], "0.1.0")

    def test_internal_dependency_versions_match_target_crates(self) -> None:
        """Every checked path dependency matches its target package version."""
        changed = sync_binding_versions.sync_rust_internal_dependency_versions(
            "0.4.0", check=True
        )
        self.assertEqual(changed, 0)


if __name__ == "__main__":
    unittest.main()
