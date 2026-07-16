#!/usr/bin/env bash
set -euo pipefail

lib_path="${1:-target/debug/libof_ffi_c.so}"

if [[ ! -f "$lib_path" ]]; then
  echo "expected shared library at '$lib_path'"
  exit 1
fi

case "$(uname -s)" in
  Linux*)
    nm_args=(-D --defined-only)
    ;;
  Darwin*)
    nm_args=(-gU)
    ;;
  *)
    echo "unsupported platform for FFI export check: $(uname -s)"
    exit 1
    ;;
esac

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mapfile -t expected_symbols < <(python3 "$script_dir/check_api_manifest.py" --emit-exports)

mapfile -t exported_symbols < <(nm "${nm_args[@]}" "$lib_path" | awk '{print $NF}' | sort -u)

missing_symbols=()
for symbol in "${expected_symbols[@]}"; do
  if ! printf '%s\n' "${exported_symbols[@]}" | grep -Fxq "$symbol"; then
    missing_symbols+=("$symbol")
  fi
done

if ((${#missing_symbols[@]} > 0)); then
  printf 'missing exported C ABI symbols:\n' >&2
  printf '  %s\n' "${missing_symbols[@]}" >&2
  exit 1
fi

echo "verified ${#expected_symbols[@]} exported C ABI symbols in $lib_path"
