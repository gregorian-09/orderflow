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

### Changed
- Rust crate publishing metadata now includes `repository`, `homepage`, and
  crate-level `documentation` links for better crates.io/docs.rs presentation.
- Workspace and binding author metadata updated to:
  Gregorian Rayne `<gregorianrayne09@gmail.com>`.

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
