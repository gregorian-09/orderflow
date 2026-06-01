# Rust Crates

Workspace crates intended for publication:

- `of_core`
- `of_execution_core`
- `of_execution`
- `of_execution_adapters`
- `of_signals`
- `of_persist`
- `of_adapters`
- `of_runtime`
- `of_ffi_c`

## Crates.io publishing order

1. `of_core`
2. `of_execution_core`
3. `of_execution`
4. `of_execution_adapters`
5. `of_signals`
6. `of_persist`
7. `of_adapters`
8. `of_runtime`
9. `of_ffi_c`

This order matches dependency topology.

## Release pipeline

Workflow: `.github/workflows/publish-rust.yml`

## Version management

Version source of truth: `bindings/versions.toml` (`bindings.rust`)  
Sync/check command: `python3 tools/release/sync_binding_versions.py --check`

The sync tool updates:

- Workspace package version in `Cargo.toml`.
- Internal crate dependency pins (`of_*` path dependencies) in `crates/*/Cargo.toml`.

## Release prerequisites

Required repository secret:

- `CARGO_REGISTRY_TOKEN`

The crates.io account behind this token must have a verified email address.
Without that, publish fails with:

`A verified email address is required to publish crates to crates.io`

## Docs

- Local docs:

```bash
cargo doc --workspace --no-deps
```

- Published docs are expected on docs.rs after crates.io release.
