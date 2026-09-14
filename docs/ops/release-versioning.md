# Release Versioning

Orderflow is a Cargo workspace with several independently versioned public
crate families. The repository also ships Python, Java, and native C SDK
artifacts. A release therefore has two different meanings:

1. The **coordinated release** identifies a tested repository state and its
   release tag.
2. A **package version** identifies the compatibility contract of one crate or
   binding artifact.

These identifiers often match for the established platform line, but they do
not have to match for an independently introduced crate. For example, the
`0.5.0` coordinated release contains the established platform crates at
`0.5.0`, the execution family at `0.2.0`, and first-release extension crates at
`0.1.0`. That is intentional semver information, not an inconsistency.

## Authoritative Files

The repository root `release.toml` is the release coordination manifest. It
records:

- the coordinated release version and tag;
- every package in the Cargo workspace, including non-publishable tools;
- each package's effective version, family, and publishability;
- the dependency-safe order for crates.io publication;
- binding package versions;
- the native C SDK version.

Cargo manifests remain authoritative for the actual package metadata. The
release manifest does not override `Cargo.toml`; it detects when the two have
drifted. `bindings/versions.toml` remains the input consumed by the existing
binding synchronizer. Its values must agree with the release manifest.

The C ABI manifest at `bindings/api_manifest.toml` is a separate compatibility
contract. It describes symbols and layouts, while `release.toml` describes
which versioned artifacts are being coordinated. A version manifest cannot
replace an ABI compatibility review.

## Reading `release.toml`

The release table is deliberately small:

```toml
[release]
coordinated_version = "0.5.0"
tag = "v0.5.0"
```

`coordinated_version` is the release-level identifier. `tag` must be the same
value prefixed with `v`; this prevents a release note, tag, and manifest from
describing different repository states.

Each package has one table:

```toml
[packages.of_execution]
version = "0.2.0"
family = "execution"
publish = true
publish_order = 10
```

`version` must equal the effective version Cargo reports for that package.
`family` groups packages for human review and release notes; it does not
change dependency resolution. `publish` states whether the package is an
artifact intended for crates.io. `publish_order` is required only for
publishable packages and starts at one with no gaps or duplicates.

The release manifest lists `replay_cli` and `perf_harness` even though they
are `publish = false`. Listing them is important: a newly added workspace
package must not silently escape release review merely because it is a binary.

The binding and artifact tables record non-Cargo outputs:

```toml
[bindings]
python = "0.5.0"
java = "0.5.0"
rust = "0.5.0"

[artifacts]
c_sdk = "0.5.0"
```

These values are checked against `bindings/versions.toml` and the
`of_ffi_c` package version. The `rust` binding value represents the established
Rust/native release line used by the binding synchronizer; independently
versioned crates are represented by their own package tables.

## Why Publication Order Is Recorded

Cargo resolves a path dependency from the local workspace, but crates.io
publication resolves it from the registry. A dependent package must therefore
be published only after the exact compatible version of its internal
dependency is visible in the registry.

The validator checks both parts of this rule:

- every local path dependency is anchored to the target package's manifest
  version (`0.5.0` or Cargo's equivalent `^0.5.0` requirement);
- the target dependency's publication slot precedes the dependent package's
  slot.

The current order is consequently a topological order, not an alphabetical
list. `of_ffi_c` is last because it composes the platform, persistence, and
execution surfaces. The tooling binaries have no publication slot because
they are operational consumers rather than registry libraries.

## Validation Commands

Run the manifest validator from the repository root:

```bash
python3 tools/release/validate_release_manifest.py
```

A successful run reports the number of workspace packages and publishable
packages. The validator obtains fresh `cargo metadata` output, so it checks
workspace membership and effective `version.workspace = true` values rather
than trusting a copied list.

To inspect the exact order used by the Rust publication workflow:

```bash
python3 tools/release/validate_release_manifest.py --print-publish-order
```

The command prints one crate name per line only after all checks pass. This
makes it suitable for a shell `mapfile` in CI without duplicating release
logic in YAML.

Run the validator's regression tests as well:

```bash
python3 tools/release/test_validate_release_manifest.py
```

The tests cover the current workspace, deterministic ordering, package version
drift, dependency requirement drift, and duplicate publication slots. The
existing binding synchronization checks remain required:

```bash
python3 tools/release/sync_binding_versions.py --check
python3 tools/release/test_sync_binding_versions.py
```

## Adding a Workspace Package

When a crate or binary is added:

1. Add it to the root workspace and give its `Cargo.toml` a deliberate
   effective version.
2. Add a `[packages.<name>]` table to `release.toml`.
3. Set `publish = true` only when the crate is a supported registry artifact.
   Publishable crates need a unique slot after all internal path dependencies.
4. Anchor every internal path dependency to the target package version.
5. Add the crate to the appropriate release documentation and README surface.
6. Run the validator, its tests, package-content checks, and the normal Rust
   compatibility gates.

An operational binary should still be listed with `publish = false`. Omitting
it makes the manifest incomplete and allows future publication automation to
miss a workspace member.

## Updating an Existing Package Family

A patch or minor package update changes only the package tables whose public
contract changed, plus any dependent path requirements that must follow the
new version. Do not change the coordinated release version merely to make
independent package versions look equal.

For a `0.y.z` package, the project still treats downstream users as real
compatibility consumers. Additive changes should remain additive, and a
breaking change requires an explicit compatibility decision, migration notes,
and the appropriate version change. The manifest is a visibility and
validation mechanism; it is not permission to bypass semver review.

## Release Workflow Boundary

CI always validates the manifest. The Rust publication workflow derives its
crate list from the validated manifest, so publication order has one source of
truth. Validation and publication are separate actions:

- ordinary CI and dry runs inspect package contents without uploading;
- publication requires the existing explicit workflow conditions and registry
  credentials;
- changing `release.toml` locally does not publish anything;
- a local branch cannot trigger GitHub Actions until it is pushed.

This separation is important when preparing a release candidate. Maintainers
can correct versions, run all local checks, and review the resulting diff
without contacting crates.io, PyPI, Maven Central, or the native artifact
release channel.

## Failure Interpretation

Treat each validator error as a release-blocking inconsistency:

- **missing workspace package** means a new member was not added to the
  manifest;
- **unknown workspace package** means a stale manifest entry remains;
- **package version mismatch** means Cargo and release metadata disagree;
- **dependency requirement drift** means a dependent package may resolve an
  unintended registry version;
- **publication order error** means ordered publication can fail or publish a
  dependent before its required registry version exists;
- **binding or SDK mismatch** means users could receive wrappers and native
  libraries from different compatibility lines.

Fix the owning manifest or source-of-truth file, then rerun validation. Do not
silence an error by removing the package from `release.toml`.
