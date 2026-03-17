# Orderflow Filesystem Registry (vcpkg)

This directory is the supported vcpkg distribution path for `orderflow-c`.

The curated-registry PR to `microsoft/vcpkg` is currently blocked on Rust/Cargo
integration policy discussion, so this repo ships a first-party filesystem
registry and overlay-port path.

## Option A: Overlay port (fastest)

Install directly from this repository without registry wiring:

```bash
VCPKG_BINARY_SOURCES=clear \
vcpkg install orderflow-c --overlay-ports=/path/to/orderflow/packaging/vcpkg/official/ports
```

## Option B: Filesystem registry (manifest mode)

1. Copy `vcpkg-configuration.example.json` into your consumer project as
   `vcpkg-configuration.json`.
2. Update:
   - `path` to your local `orderflow/packaging/vcpkg/registry` path
   - `default-registry.baseline` to a real commit from your vcpkg checkout
3. Add dependency in your `vcpkg.json`:

```json
{
  "dependencies": ["orderflow-c"]
}
```

Then install in manifest mode (recommended for CI/smoke runs):

```bash
VCPKG_BINARY_SOURCES=clear vcpkg install
```

`VCPKG_BINARY_SOURCES=clear` disables binary-cache upload/download, which
avoids non-fatal cache-submission warnings in ephemeral environments.

## Registry layout

- `ports/orderflow-c/0.1.1/*` contains the port recipe files.
- `versions/baseline.json` sets default version resolution.
- `versions/o-/orderflow-c.json` maps versions to filesystem paths.
