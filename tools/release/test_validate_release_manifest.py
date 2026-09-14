#!/usr/bin/env python3
"""Unit tests for coordinated release manifest validation."""

from __future__ import annotations

import copy
import unittest

import validate_release_manifest


class ValidateReleaseManifestTests(unittest.TestCase):
    """Protect package, dependency, and publication-order invariants."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = validate_release_manifest.load_manifest()
        cls.metadata = validate_release_manifest.load_cargo_metadata()
        cls.bindings = validate_release_manifest.load_binding_versions()

    def test_current_workspace_is_valid(self) -> None:
        """The checked-in manifest describes every current workspace package."""
        self.assertEqual(
            validate_release_manifest.validate_manifest(
                self.manifest, self.metadata, self.bindings
            ),
            [],
        )

    def test_published_packages_follow_declared_order(self) -> None:
        """The publication list is deterministic and excludes tooling binaries."""
        self.assertEqual(
            validate_release_manifest.published_package_names(self.manifest),
            [
                "of_core",
                "of_analytics",
                "of_execution_core",
                "of_fix",
                "of_signals",
                "of_persist",
                "of_persist_parquet",
                "of_adapters",
                "of_runtime",
                "of_execution",
                "of_execution_algos",
                "of_execution_adapters",
                "of_ffi_c",
            ],
        )

    def test_version_drift_is_reported(self) -> None:
        """A package version change cannot bypass the manifest check."""
        manifest = copy.deepcopy(self.manifest)
        manifest["packages"]["of_core"]["version"] = "0.6.0"
        errors = validate_release_manifest.validate_manifest(
            manifest, self.metadata, self.bindings
        )
        self.assertTrue(any("of_core version" in error for error in errors))

    def test_dependency_drift_is_reported(self) -> None:
        """A local dependency must stay anchored to its target release version."""
        metadata = copy.deepcopy(self.metadata)
        analytics = next(
            package for package in metadata["packages"] if package["name"] == "of_analytics"
        )
        dependency = next(
            dependency
            for dependency in analytics["dependencies"]
            if dependency["name"] == "of_core"
        )
        dependency["req"] = "^0.4.0"
        errors = validate_release_manifest.validate_manifest(
            self.manifest, metadata, self.bindings
        )
        self.assertTrue(any("of_analytics depends on of_core" in error for error in errors))

    def test_duplicate_publication_order_is_reported(self) -> None:
        """Two crates cannot silently compete for one publication slot."""
        manifest = copy.deepcopy(self.manifest)
        manifest["packages"]["of_signals"]["publish_order"] = 1
        errors = validate_release_manifest.validate_manifest(
            manifest, self.metadata, self.bindings
        )
        self.assertTrue(any("publication order 1 is shared" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
