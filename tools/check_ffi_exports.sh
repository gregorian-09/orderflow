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

expected_symbols=(
  of_api_version
  of_build_info
  of_engine_create
  of_engine_start
  of_engine_stop
  of_engine_destroy
  of_subscribe
  of_unsubscribe
  of_unsubscribe_symbol
  of_reset_symbol_session
  of_ingest_trade
  of_ingest_book
  of_configure_external_feed
  of_external_set_reconnecting
  of_external_health_tick
  of_get_book_snapshot
  of_get_analytics_snapshot
  of_get_derived_analytics_snapshot
  of_get_signal_snapshot
  of_get_metrics_json
  of_string_free
  of_engine_poll_once
)

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
