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

- Native release artifacts are produced by:
  - `.github/workflows/release-native-artifacts.yml`
- Consumers should bundle the header and matching native binary.

## API documentation source

- `docs/handbook/05-api-reference.md`
- `docs/api/README.md`
