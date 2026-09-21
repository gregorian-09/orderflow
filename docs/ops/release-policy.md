# Release Policy

This document defines how Orderflow moves a change from an accepted pull
request to a supported public release. It is the policy layer above the
machine-readable [`release.toml`](release-versioning.md#authoritative-files)
manifest and the
command-by-command [`release_checklist.md`](release_checklist.md).

The policy exists because Orderflow is a multi-artifact library, not a single
binary. A release can contain Rust crates, a C ABI, a native SDK, Python and
Java bindings, documentation, and operational tools. These artifacts must be
released from the same reviewed source state even when their package versions
are intentionally different.

## Policy Goals

The release process has six goals:

1. Keep `main` buildable, testable, and releasable.
2. Preserve existing Rust, C, Python, and Java contracts by default.
3. Publish only artifacts that have passed the complete compatibility and
   packaging gates.
4. Make each package family independently understandable and versionable.
5. Produce enough evidence that a release can be reproduced and audited.
6. Prefer a forward fix over deletion, yanking, or rewriting of a published
   artifact.

Release frequency is subordinate to correctness. A release is not made merely
because a calendar date arrived, and a release is not delayed when a verified
security issue requires an out-of-band fix.

## Release Streams

Orderflow uses four release streams. The stream determines the amount of
coordination and review required; it does not permit a public API break.

| Stream | Normal interval | Use | Compatibility expectation |
| --- | --- | --- | --- |
| Patch | As needed, normally every 1-2 weeks when fixes accumulate | Bugs, security fixes, documentation corrections, performance fixes with unchanged behavior, and packaging repairs | No intentional public API, ABI, serialized-format, or binding break |
| Feature | Every 4-6 weeks | Additive capabilities that are complete, documented, tested, and supportable | Existing contracts remain valid; new APIs are additive and opt-in where appropriate |
| Release candidate | 1-2 weeks before a feature release | Stabilization, downstream testing, packaging verification, and documentation review | Only release-blocking fixes; no new feature scope |
| Security or emergency | As soon as the fix is verified | Exploitable defects, data-loss risk, or a release-blocking production defect | Smallest compatible change possible, with a public advisory when appropriate |

The feature interval is a release train, not a promise that a fixed number of
features will ship. An incomplete feature remains on its branch or is removed
from the release scope; it is not hidden behind an unreviewed partial API.

## Version Semantics

Orderflow follows Semantic Versioning while applying a stricter compatibility
contract during the pre-1.0 phase. The project treats downstream users as real
compatibility consumers even when a crate version begins with `0`.

### Coordinated release version

The `[release]` table in `release.toml` identifies the tested repository state:

```toml
[release]
coordinated_version = "0.5.0"
tag = "v0.5.0"
```

The coordinated version is the version of the release train. It is not a
requirement that every crate or binding use the same package version. The
release tag, release notes, changelog entry, documentation version, and
workflow ref must describe the same source state.

### Package versions

Package versions are owned by their package manifests and checked against
`release.toml`. Increment a package only when its public contract, dependency
requirements, packaging, or supported behavior requires a new version.

Use these rules:

- **Patch** (`0.5.1`): compatible bug fixes, security fixes, documentation or
  packaging repairs, and measured performance improvements that preserve
  behavior.
- **Minor feature** (`0.6.0` before 1.0): additive public capability or a
  coordinated feature-family expansion. Existing callers must continue to
  compile and retain their previous behavior.
- **Breaking pre-1.0 change**: requires an explicit compatibility decision,
  migration documentation, semver review, and the next intentional release
  line. It must never be smuggled into a patch release. The default decision is
  to add a new API and deprecate the old one instead.
- **Post-1.0 major** (`1.0.0`, `2.0.0`, and so on): reserved for an approved
  breaking contract change after a deprecation and migration period.

Independent crates such as `of_fix`, `of_analytics`, and the execution-family
crates keep their own package version histories. A new crate starts at
`0.1.0`; an additive change after its first publication is normally `0.1.1`
for a compatible fix or `0.2.0` for a feature release. The exact decision is
recorded in `release.toml` and the release notes rather than inferred from the
coordinated version.

### What counts as a public contract

The compatibility review covers more than Rust function signatures:

- exported Rust types, traits, methods, constants, feature names, and error
  behavior;
- `repr(C)` layouts, C symbol names, enum values, ownership, buffer rules, and
  error codes;
- Python and Java names, argument behavior, exceptions, lifecycle, and native
  loading rules;
- serialized records, WAL frames, checkpoints, replay ordering, and schema
  migration rules;
- CLI names, flags, exit codes, output schemas, configuration keys, and
  dashboard routes;
- package names, supported platforms, feature combinations, and documented
  operational defaults.

An implementation refactor is release-safe only when these contracts remain
valid and the relevant tests demonstrate that fact.

## Package-Family Coordination

The release manifest groups artifacts for review without forcing every group
to share a version:

- **Platform**: `of_core`, `of_signals`, `of_persist`, `of_adapters`,
  `of_runtime`, and `of_ffi_c`.
- **Analytics**: `of_analytics` and its optional heavy-analysis surface.
- **Persistence**: `of_persist_parquet` and cold-storage tooling.
- **Execution**: `of_execution_core`, `of_execution`,
  `of_execution_algos`, `of_execution_adapters`, and `of_fix`.
- **Bindings and native artifacts**: Python, Java, the Rust binding metadata,
  and the C SDK.
- **Operational tools**: `replay_cli` and `perf_harness`, which are reviewed
  but are not registry packages.

When a dependency version changes, publish every dependent package whose
registry dependency must point at the new version, even if its source API did
not change. Do not bump unrelated package families merely to make the version
table look uniform.

The publication order is generated from `release.toml`:

```bash
python3 tools/release/validate_release_manifest.py --print-publish-order
```

Never duplicate that order in a release issue or edit it only in a workflow.
The manifest validator checks workspace membership, effective versions,
anchored internal requirements, binding versions, artifact versions, and
dependency-safe ordering.

## Standard Release Lifecycle

The lifecycle is intentionally predictable. Dates are relative to the planned
release date and can be shortened for a security release.

### T-42 to T-14: feature train

Changes are implemented on short-lived branches and merged through pull
requests. Each change must have an owner, a crate boundary, compatibility
notes, tests, and documentation. `main` remains the integration branch.

During this period:

- public API additions are additive and independently reviewable;
- experimental behavior is feature-gated or kept private until complete;
- generated files are refreshed by their owning tools;
- performance-sensitive changes include a benchmark or a reason why one is
  not meaningful;
- changes that affect more than one binding include parity tests;
- no package is published from a feature branch.

### T-14: feature freeze

The release owner freezes feature scope. Pull requests may still merge when
they are documentation, test, packaging, or release-blocking fixes, but a new
public feature moves to the next train unless the release owner records an
exception and repeats the affected gates.

At the freeze point, prepare:

1. package versions and anchored internal dependency requirements;
2. the coordinated version and tag in `release.toml`;
3. `CHANGELOG.md` and the versioned release notes;
4. crate, binding, native SDK, and handbook documentation;
5. the release evidence checklist and known limitations.

### T-14 to T-7: release candidate preparation

Run the complete local validation suite from a clean tree. Inspect package
contents rather than relying only on workspace compilation. Build the C ABI,
Python wheel/sdist, Java artifact, documentation, and operational binaries.

The release owner then creates a release-candidate commit. The candidate is
reviewed as a release artifact, not merely as a source diff. Downstream users
should test the candidate against the public APIs they use.

### T-7 to T0: stabilization

Create a release candidate tag only after the candidate commit is on the
release source branch. The stabilization period permits only:

- correctness fixes;
- security fixes;
- documentation corrections that describe shipped behavior;
- packaging or workflow repairs;
- test fixes that expose a real release risk.

Every candidate fix reruns the affected focused gates and the complete suite
before the final tag. If the candidate contract changes materially, create a
new candidate and restart the stabilization window.

### T0: final tag and publication

The final tag is created from the reviewed commit after all gates pass. The
release owner records the commit, tag, manifest, toolchain, lockfile, and
validation results before uploading anything.

Publication is explicit and ordered:

1. publish dependency-root Rust crates;
2. wait for registry visibility and publish dependent crates in manifest order;
3. build and publish bindings against the exact native ABI release;
4. build and attach native C SDK artifacts with checksums;
5. update downstream registries such as the vcpkg registry;
6. activate the matching documentation version.

Build and dry-run jobs may run from `main` or a release candidate. Upload jobs
must run from the approved release tag and require the repository's configured
environment approval and credentials. A workflow that can upload from an
arbitrary branch is a release-process defect.

### T+1 to T+7: verification and support

After publication, verify each artifact from a clean temporary directory:

- resolve every published Rust crate at the recorded version;
- install the Python distribution and run the binding smoke tests;
- resolve the Java artifact and run the example/smoke test;
- download the native SDK and verify checksums, headers, libraries, and
  `pkg-config` metadata;
- confirm the documentation version, links, examples, and generated indexes;
- record registry URLs, workflow run IDs, artifact hashes, and any warnings.

The release is complete only after these checks pass. Update the release notes
with any operational limitation discovered during verification.

## Required Release Gates

The full command list lives in [`release_checklist.md`](release_checklist.md).
The gates below define why each category is required.

### Source and manifest gates

- clean worktree and approved commit;
- no untracked generated or local-only files;
- `release.toml` agrees with Cargo metadata;
- binding versions and native SDK version agree with the selected release;
- publication order is complete, unique, and dependency-safe;
- lockfile and declared toolchain are present and reproducible.

### Compatibility gates

- semver checks use the previous established package baseline;
- C ABI manifest, header, generated bindings, and exported symbols agree;
- existing Python and Java APIs retain names, signatures, lifecycle, and error
  behavior;
- serialized and persisted formats have round-trip, corruption, ordering, and
  migration evidence;
- CLI and dashboard contracts retain documented behavior.

### Behavior and performance gates

- default, no-default, individual provider, and all-feature builds pass;
- unit, integration, replay, recovery, binding, and end-to-end tests pass;
- clippy, rustdoc, formatting, documentation coverage, and generated-file
  checks pass;
- measured hot paths remain bounded and do not acquire accidental blocking,
  unbounded allocation, or hidden network work;
- failure, shutdown, reconnect, duplicate, out-of-order, and backpressure
  paths are tested where the affected component supports them.

### Supply-chain and artifact gates

- dependency, license, advisory, and source-policy checks pass;
- package contents contain the intended README, license, source, and metadata;
- Python wheels and sdist install in a clean environment;
- Java artifacts contain required metadata and signatures;
- native artifacts contain headers, libraries, metadata, checksums, and the
  supported platform matrix.

## Release Evidence

A release owner keeps one evidence record containing:

- release version, tag, commit, and UTC timestamps;
- output of the release-manifest and binding synchronization checks;
- toolchain versions and lockfile identity;
- CI workflow URLs and successful job IDs;
- package names, versions, registry URLs, and checksums;
- native SDK platform/architecture matrix;
- documentation URL and activated version;
- known limitations, skipped checks, and their owners;
- post-release verification results.

Evidence may live in the release description or an internal release record,
but secrets, credentials, private certificates, customer data, and restricted
provider material must never be included.

## Partial Publication and Recovery

Published registry versions are immutable. If one artifact fails after another
has uploaded:

1. stop the publication workflow;
2. record the exact successful and failed artifacts;
3. do not delete or replace the successful version;
4. diagnose registry visibility, credentials, packaging, and dependency state;
5. resume idempotently only after the failing condition is understood;
6. publish a compatible forward fix if the artifact itself is defective;
7. update release notes and downstream version instructions.

Do not publish a dependent crate that cannot resolve the exact compatible
dependency version. Do not force users to mix an unverified native library
with a different binding release. When a release cannot be completed safely,
mark it incomplete, communicate the affected artifacts, and prepare the next
compatible patch release.

## Security Releases

Security fixes may bypass the normal feature-train interval, but they do not
bypass validation. Keep the patch minimal, avoid unrelated refactors, test the
exploitation boundary and the regression boundary, and coordinate disclosure
with affected users when the issue is not already public.

Security releases must still update the changelog, package versions, release
manifest, affected README/API documentation, and release evidence. If a
credential, certificate, or private transcript was exposed, rotate it; never
place the replacement secret in the repository or release evidence.

## Maintainer Responsibilities

The release owner is responsible for scope, version decisions, evidence, and
the final go/no-go decision. Crate owners are responsible for compatibility,
tests, documentation, and known limitations in their area. CI is evidence, not
the owner of engineering judgment: a green job cannot make an undocumented
breaking change acceptable.

A release may be delayed when any required gate is unavailable, ambiguous, or
not reproducible. The decision and the missing evidence are recorded so the
next maintainer does not have to reconstruct the reasoning from chat history.

## External References

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Cargo publish](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
- [GitHub deployment environments](https://docs.github.com/en/actions/concepts/workflows-and-actions/deployment-environments)
