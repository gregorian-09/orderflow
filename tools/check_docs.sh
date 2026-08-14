#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${root_dir}"

python3 tools/docs_coverage.py --enforce
python3 tools/generate_docs_inventory.py --check
python3 tools/generate_rust_surface.py --check
python3 tools/generate_rust_values.py --check
python3 tools/generate_binding_surface.py --check
python3 tools/generate_package_matrix.py --check
python3 tools/generate_crate_pages.py --check
python3 tools/enrich_api_reference.py --check
python3 tools/enrich_handbook_public_lists.py --check

rust_target_dir="$(mktemp -d "${TMPDIR:-/tmp}/orderflow-rust-docs.XXXXXX")"
site_dir="$(mktemp -d "${TMPDIR:-/tmp}/orderflow-mkdocs.XXXXXX")"
cleanup() {
    rm -rf "${rust_target_dir}" "${site_dir}"
}
trap cleanup EXIT

cargo doc --workspace --all-features --no-deps --target-dir "${rust_target_dir}"

if [[ -x "${root_dir}/.venv/bin/mkdocs" ]]; then
    mkdocs_cmd="${root_dir}/.venv/bin/mkdocs"
else
    mkdocs_cmd="mkdocs"
fi
"${mkdocs_cmd}" build --strict --site-dir "${site_dir}"

printf 'documentation checks passed\n'
