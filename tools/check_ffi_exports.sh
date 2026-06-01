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
  of_execution_api_version
  of_execution_engine_create
  of_execution_engine_start
  of_execution_engine_stop
  of_execution_engine_destroy
  of_execution_submit_order
  of_execution_cancel_order
  of_execution_amend_order
  of_execution_poll
  of_execution_get_order_state
  of_execution_health
  of_execution_metrics
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
of_get_book_analytics_snapshot
of_compute_weighted_average_price
of_compute_depth_slope
of_get_mid_price
of_get_effective_spread_bps
of_get_half_spread_cost_bps
of_get_realised_spread_bps
of_get_book_event_analytics
of_get_resiliency_snapshot
of_get_vpin_snapshot
of_get_kyle_lambda_snapshot
of_get_amihud_snapshot
of_get_cvd_enhancement_snapshot
of_get_pattern_snapshot
of_get_volatility_snapshot
of_get_noise_snapshot
of_get_hasbrouck_snapshot
of_get_almgren_chriss_snapshot
of_get_spread_decomp_snapshot
of_get_acd_snapshot
of_get_regime_snapshot
of_get_kinetic_energy_snapshot
of_get_dark_pool_snapshot
of_get_options_flow_snapshot
  of_get_futures_snapshot
  of_compute_lob_features
  of_engine_set_analytics_config
  of_get_vol_signature_snapshot
  of_get_agent_type_snapshot
  of_get_dark_lit_correlation_snapshot
  of_get_institutional_flow_snapshot
  of_get_oi_analysis_snapshot
  of_get_analytics_snapshot
  of_get_bar_series          # requires feature "tickbar"
  of_get_derived_analytics_snapshot
  of_get_session_candle_snapshot
  of_get_interval_candle_snapshot
  of_get_signal_snapshot
  of_get_metrics_json
  of_string_free
  of_engine_poll_once
  of_engine_set_tickbar_interval   # requires feature "tickbar"
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
