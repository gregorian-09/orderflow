# Orderflow Engine and Bindings

Orderflow is a multi-language market data and analytics engine with:

- Rust core/runtime crates
- C ABI
- Python binding (`ctypes`)
- Java binding (JNA)
- Dashboard and tooling for live/replay workflows

## Documentation

Start with:

- [docs/README.md](./docs/README.md)
- [docs/handbook/README.md](./docs/handbook/README.md)
- [docs/bindings/README.md](./docs/bindings/README.md)

## Quick Build

```bash
cargo build
cargo test
```

Build C ABI for bindings:

```bash
cargo build -p of_ffi_c
```

## Distribution and Publishing

Release runbook:

- [docs/ops/distribution_release.md](./docs/ops/distribution_release.md)

Automated workflows:

- `.github/workflows/release-native-artifacts.yml`
- `.github/workflows/publish-rust.yml`
- `.github/workflows/publish-java.yml`
- `.github/workflows/publish-python.yml`
