# Official vcpkg Submission (orderflow-c)

Status: upstream curated-registry submission is currently blocked by maintainer
feedback on Rust/Cargo integration strategy:

- PR: `microsoft/vcpkg#50493`
- Reviewer note: per-port Rust bootstrap/cargo recipe is not considered a
  long-term integration solution.

Use the supported first-party path instead:

- `packaging/vcpkg/registry` (filesystem registry)
- or overlay install via `packaging/vcpkg/official/ports`

## Contents

- `ports/orderflow-c/portfile.cmake`
- `ports/orderflow-c/vcpkg.json`
- `ports/orderflow-c/usage`

## Upstream submission steps (archived)

1. Clone upstream vcpkg and create a branch:
   - `git clone https://github.com/microsoft/vcpkg.git`
   - `cd vcpkg && git checkout -b add-orderflow-c`
2. Copy this repo's `packaging/vcpkg/official/ports/orderflow-c` into
   `vcpkg/ports/orderflow-c`.
3. Validate locally:
   - `./vcpkg format-manifest ports/orderflow-c/vcpkg.json`
   - `./vcpkg x-add-version orderflow-c`
   - `VCPKG_BINARY_SOURCES=clear ./vcpkg install orderflow-c`
4. Commit generated files:
   - `ports/orderflow-c/*`
   - `versions/o-/orderflow-c.json`
   - `versions/baseline.json`
5. Open PR to `microsoft/vcpkg`.

## Notes

- This port currently targets `x64` triplets on Windows/Linux/macOS.
- Source reference is pinned to tag `v0.1.1` with SHA512 in `portfile.cmake`.
- If `bindings.rust` changes, update:
  - `REF` + `SHA512` in `portfile.cmake`
  - `version-semver` in `vcpkg.json`
