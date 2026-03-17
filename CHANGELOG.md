# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]
### Added
- Rust crate front-page documentation (`//!`) for `of_core`, `of_signals`,
  `of_persist`, `of_adapters`, `of_runtime`, and `of_ffi_c`, including
  purpose, architecture notes, and quick-start examples.
- C ABI API documentation comments for exported `of_ffi_c` symbols and FFI
  structs to improve docs.rs discoverability for non-Rust integrators.
- Java package-level JavaDoc (`package-info.java`) for
  `com.orderflow.bindings` and `com.orderflow.examples`.
- Richer Python module-level API documentation for `orderflow.api`,
  `orderflow._ffi`, and package root `orderflow`.
- JavaDoc overview page (`bindings/java/src/main/javadoc/overview.html`) to
  provide a richer published API landing page for Maven consumers.
- C SDK distribution packaging in `.github/workflows/release-native-artifacts.yml`,
  now publishing versioned platform archives with header, libraries, pkg-config
  metadata, and SDK README.
- C API header constants for stream kinds and data-quality flags
  (`of_stream_kind_t`, `of_data_quality_flags_t`) for first-class C developer
  ergonomics.
- C usage example: `examples/c/basic.c`.
- Official vcpkg submission scaffold for C consumers:
  `packaging/vcpkg/official/ports/orderflow-c` with portfile, manifest,
  and usage docs.

### Changed
- Rust crate publishing metadata now includes `repository`, `homepage`, and
  crate-level `documentation` links for better crates.io/docs.rs presentation.
- Workspace and binding author metadata updated to:
  Gregorian Rayne `<gregorianrayne09@gmail.com>`.
- Python PyPI metadata now includes project URLs, classifiers, and keywords.
- Java Maven metadata now includes organization, issue tracker URL,
  inception year, and developer id/email.
- Binding release versions are now managed centrally in
  `bindings/versions.toml` and synchronized via
  `tools/release/sync_binding_versions.py` across Python, Java, and Rust/C
  package version surfaces.
- Python and Java binding package descriptions were upgraded with richer
  packaging-facing docs (badges, API map, operations notes, and direct doc links)
  to improve PyPI and Maven discoverability.
- Binding versions were prepared for this release cycle: Python `0.1.3`,
  Java `0.1.2`, Rust/C updated to `0.1.2`.
- Added root `LICENSE` file (MIT) to satisfy package distribution and
  registry compliance requirements.

## [0.1.1] - 2026-03-16
### Fixed
- Python binding: `Engine.subscribe(..., callback=None)` now works correctly by
  passing a typed null callback pointer to the C ABI, instead of raising a
  `ctypes.ArgumentError`.

## [0.1.0] - 2026-03-09
### Added
- Initial public release of Rust crates (`of_core`, `of_signals`, `of_persist`,
  `of_adapters`, `of_runtime`, `of_ffi_c`), Java binding
  (`io.github.gregorian-09:orderflow-java-binding`), and Python binding
  (`orderflow-gregorian09`).
