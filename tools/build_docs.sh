#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
site_dir="${1:-${root_dir}/site}"
rust_target_dir="${root_dir}/target/docs"

cd "${root_dir}"

python3 tools/generate_docs_inventory.py
python3 tools/generate_rust_surface.py
python3 tools/generate_rust_values.py
python3 tools/generate_binding_surface.py
python3 tools/generate_package_matrix.py
python3 tools/generate_crate_pages.py
cargo doc --workspace --all-features --no-deps --target-dir "${rust_target_dir}"

if [[ -x "${root_dir}/.venv/bin/mkdocs" ]]; then
    mkdocs_cmd="${root_dir}/.venv/bin/mkdocs"
else
    mkdocs_cmd="mkdocs"
fi
"${mkdocs_cmd}" build --strict --site-dir "${site_dir}"

mkdir -p "${site_dir}/reference/rust"
cp -a "${rust_target_dir}/doc/." "${site_dir}/reference/rust/"

printf 'Documentation site written to %s\n' "${site_dir}"
