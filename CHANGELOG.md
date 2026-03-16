# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
