# Rust Crates

Workspace crates intended for publication:

- `of_core`
- `of_signals`
- `of_persist`
- `of_adapters`
- `of_runtime`
- `of_ffi_c`

## Crates.io publishing order

1. `of_core`
2. `of_signals`
3. `of_persist`
4. `of_adapters`
5. `of_runtime`
6. `of_ffi_c`

This order matches dependency topology.

## Release pipeline

Workflow: `.github/workflows/publish-rust.yml`

## Docs

- Local docs:

```bash
cargo doc --workspace --no-deps
```

- Published docs are expected on docs.rs after crates.io release.
