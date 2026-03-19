# C ABI

Header: `crates/of_ffi_c/include/orderflow.h`  
Library crate: `crates/of_ffi_c`

## Build

```bash
cargo build -p of_ffi_c --release
```

Output:

- Linux: `target/release/libof_ffi_c.so`
- macOS: `target/release/libof_ffi_c.dylib`
- Windows: `target/release/of_ffi_c.dll`

## Distribution

- C SDK release artifacts are produced by:
  - `.github/workflows/release-native-artifacts.yml`
- Each release publishes platform bundles:
  - `orderflow-c-sdk-<os>-<arch>-<version>.tar.gz`
- Bundle layout:
  - `include/orderflow.h`
  - `lib/` (shared library + static/import library when available)
  - `pkgconfig/orderflow.pc`
  - `README.md`

## vcpkg install paths

Primary supported path:

- Git registry: `https://github.com/gregorian-09/orderflow-vcpkg-registry`

In-repo local alternatives:

- Filesystem registry snapshot: `packaging/vcpkg/registry`

Fast local path:

- Overlay port: `packaging/vcpkg/official/ports/orderflow-c`

Example overlay install:

```bash
VCPKG_BINARY_SOURCES=clear \
vcpkg install orderflow-c --overlay-ports=/path/to/orderflow/packaging/vcpkg/official/ports
```

Example filesystem registry (`vcpkg-configuration.json`):

```json
{
  "default-registry": {
    "kind": "builtin",
    "baseline": "<replace-with-vcpkg-commit>"
  },
  "registries": [
    {
      "kind": "filesystem",
      "path": "/absolute/path/to/orderflow/packaging/vcpkg/registry",
      "packages": ["orderflow-c"]
    }
  ]
}
```

Example Git registry (`vcpkg-configuration.json`):

```json
{
  "default-registry": {
    "kind": "builtin",
    "baseline": "<replace-with-vcpkg-commit>"
  },
  "registries": [
    {
      "kind": "git",
      "repository": "https://github.com/gregorian-09/orderflow-vcpkg-registry.git",
      "baseline": "82c338f9a665990e5bdbf38e86b33e7c6410db97",
      "packages": ["orderflow-c"]
    }
  ]
}
```

Curated registry status:

- Upstream PR: `microsoft/vcpkg#50493`
- Current maintainer feedback blocks merge until vcpkg has an accepted Rust
  integration approach beyond per-port cargo bootstrap.

Note:

- For local smoke tests/CI, set `VCPKG_BINARY_SOURCES=clear` to avoid
  non-fatal binary-cache submission warnings.

## Using the C SDK

Extract a release bundle and point `PKG_CONFIG_PATH` to the included pkg-config
metadata.

```bash
tar -xzf orderflow-c-sdk-linux-x86_64-<version>.tar.gz -C /opt/orderflow-c-sdk
export PKG_CONFIG_PATH=/opt/orderflow-c-sdk/pkgconfig
cc -O2 examples/c/basic.c -o basic $(pkg-config --cflags --libs orderflow)
```

Runtime loader hints for dynamic linking:

- Linux: set `LD_LIBRARY_PATH` to include bundle `lib/`.
- macOS: set `DYLD_LIBRARY_PATH` to include bundle `lib/`.
- Windows: place `of_ffi_c.dll` next to the executable or add bundle `lib\` to `PATH`.

## Version management

The C ABI library crate (`of_ffi_c`) uses the Rust workspace version and is
therefore governed by `bindings/versions.toml` (`bindings.rust`).

Sync/check command:

```bash
python3 tools/release/sync_binding_versions.py --check
```

## API documentation source

- `docs/handbook/05-api-reference.md`
- `docs/api/README.md`
- `examples/c/basic.c`
