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

## 3) Enforce docs/API coverage gates

```bash
python3 tools/docs_coverage.py --enforce
```

## 4) Build/test before publish

```bash
cargo test -q
mvn -B -f bindings/java/pom.xml -DskipTests package
python3 -m build bindings/python --sdist --wheel --outdir /tmp/orderflow-py-dist
```

## 5) Publish workflows

Trigger repository publish workflows for Rust, Python, Java, and native
artifacts, then verify package index availability.
