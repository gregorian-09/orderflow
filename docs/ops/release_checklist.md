# Release Checklist

This checklist covers repeatable release tasks for package/version publishing.

## 1) Sync versions

```bash
python3 tools/release/sync_binding_versions.py
```

Validation-only mode:

```bash
python3 tools/release/sync_binding_versions.py --check
python3 tools/release/test_sync_binding_versions.py
```

The tool reads each internal path dependency's target crate manifest. Crates
using `version.workspace = true` follow the established binding/native release
line, while explicitly versioned new crates retain their independent release
line. Do not add per-crate hard-coded overrides when a new crate is introduced.

## 2) Sync vcpkg Git registry baseline

After pushing updates to `gregorian-09/orderflow-vcpkg-registry`, update local
docs/config references to the published registry `HEAD`:

```bash
python3 tools/release/sync_vcpkg_registry_baseline.py
```

Validation-only mode:

```bash
python3 tools/release/sync_vcpkg_registry_baseline.py --check
```

Optional explicit SHA:

```bash
python3 tools/release/sync_vcpkg_registry_baseline.py --sha <40-char-sha>
```

## 3) Enforce generated API and documentation gates

```bash
python3 tools/check_api_manifest.py
python3 tools/generate_binding_signatures.py --check
python3 tools/test_generate_binding_signatures.py
python3 tools/check_binding_parity.py
python3 tools/generate_api_inventory.py --check
python3 tools/docs_coverage.py --enforce
```

## 4) Validate Rust behavior and quality

```bash
cargo fmt --check
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
cargo +1.88.0 check --workspace --all-features --locked
cargo audit
cargo deny check
```

The declared MSRV is the minimum toolchain for the complete feature graph, not
only for default features. `cargo-deny` may report reviewed duplicate-version
warnings; advisories, licenses, bans, or source failures must fail the release.

## 5) Validate ABI and bindings

```bash
cargo build -p of_ffi_c --locked
./tools/check_ffi_exports.sh target/debug/libof_ffi_c.so
python3 -m py_compile bindings/python/orderflow/*.py
python3 tools/smoke_python_binding.py
PYTHONPATH=bindings/python python3 -m unittest discover -s bindings/python/tests -v
mvn -B -f bindings/java/pom.xml -DskipTests package
mvn -q -f bindings/java/pom.xml -DskipTests dependency:copy-dependencies -DincludeScope=runtime
java -cp 'bindings/java/target/classes:bindings/java/target/dependency/*' \
  com.orderflow.examples.BindingSmokeExample
python3 -m build bindings/python --sdist --wheel --outdir /tmp/orderflow-py-dist
```

Keep the Python/Java package version and native `of_ffi_c` version synchronized.

## 6) Enforce the additive Rust API contract

```bash
for crate in of_core of_adapters of_signals of_persist of_runtime of_ffi_c; do
  cargo semver-checks check-release \
    -p "$crate" \
    --baseline-rev v0.4.0 \
    --release-type minor
done
```

Use the previous established release tag as the baseline. New `0.1.x` crates
enter this matrix after their first registry release establishes a baseline.

## 7) Inspect Rust package contents

```bash
for crate in of_core of_analytics of_execution_core of_fix of_signals \
  of_persist of_persist_parquet of_adapters of_runtime of_execution \
  of_execution_algos of_execution_adapters of_ffi_c; do
  cargo check --locked -p "$crate"
  cargo package --allow-dirty --list -p "$crate"
done

cargo package --allow-dirty -p of_core
```

`cargo package --list` validates package metadata and included files without
requiring unpublished internal versions to exist on crates.io. Cargo performs
full package verification for each downstream crate during ordered publication
after its new internal dependencies become visible in the registry.

## 8) Publish workflows

Trigger repository publish workflows for Rust, Python, Java, and native
artifacts only after creating the release tag and approving publication. A
normal `main` push runs build/package verification; Rust and Java publication
requires explicit workflow dispatch, while Python upload requires a release tag
or explicit workflow dispatch. Verify package-index availability and artifact
checksums before updating downstream registries.
