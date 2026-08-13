# Documentation Charter

## Purpose

Orderflow documentation is a product surface for an open-source library. It
must describe the system from first principles through exact implementation
contracts without breaking the distinction between explanation, procedure,
reference, and operations.

## Audience Paths

| Audience | Primary path | Required outcome |
| --- | --- | --- |
| Analyst or researcher | Start Here -> Learn Orderflow -> Cookbooks | Understand events, analytics, replay, and validation |
| Rust integrator | Start Here -> Architecture -> Rust Reference | Build against stable crate APIs |
| C, Python, or Java integrator | Bindings -> Cookbooks -> Compatibility | Load the native runtime and handle ownership correctly |
| Execution integrator | OMS -> Execution Connectivity -> Operations | Route, monitor, recover, and reconcile orders |
| Operator | Operations -> Recovery -> Compatibility | Deploy and recover the system safely |
| Contributor | Architecture -> Reference -> Contributors | Extend the project without violating contracts |

## Normative Language

- **Must** describes a compatibility, safety, or correctness requirement.
- **Should** describes the supported recommendation.
- **May** describes an extension or optional behavior.
- **Implementation detail** identifies behavior users must not depend on unless
  the public contract says otherwise.

## Page Completion Contract

An important page is complete only when it covers purpose, prerequisites,
inputs, outputs, units, invariants, lifecycle, errors, ownership, thread
safety, latency and allocation behavior, persistence implications, examples,
version availability, and related references.

## API Documentation Rule

The source code and canonical manifests define exact API names, layouts, values,
and signatures. Narrative documentation explains those contracts but does not
invent behavior. Any ambiguity must be resolved in code and tests before it is
presented as a guarantee.

## Versioning

- `latest` documents the development branch.
- `stable` documents the latest supported release.
- Release versions document the exact tagged source state.
- New behavior identifies the version in which it was introduced.
- Breaking changes require an explicit migration page and compatibility note.

## Quality Gates

The documentation build will eventually enforce:

- Generated inventory freshness.
- Rust, C, Python, and Java API coverage.
- Valid internal links and anchors.
- Valid Mermaid diagrams.
- Compiled or tested code examples.
- No undocumented public values or configuration defaults.
- No stale version claims.
