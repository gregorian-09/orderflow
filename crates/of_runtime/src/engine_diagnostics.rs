use super::*;

pub(crate) fn escape_json(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

pub(crate) fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn optional_usize_json(value: Option<usize>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn optional_str_json(value: Option<&str>) -> String {
    value
        .map(|v| format!("\"{}\"", escape_json(v)))
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn format_runtime_market_data_persistence_json(
    persistence: Option<&RuntimeMarketDataPersistence>,
) -> String {
    let Some(persistence) = persistence else {
        return "{\"schema_version\":1,\"mode\":\"disabled\",\"enabled\":false,\"degraded\":false,\"memory_only\":false,\"failure_action\":\"mark_degraded\",\"blocks_trading\":false,\"accepted_records\":0,\"written_records\":0,\"abandoned_records\":0,\"records_lag\":0,\"event_time_lag_ns\":0,\"latest_accepted_ts_recv_ns\":0,\"latest_written_ts_recv_ns\":0,\"queue_depth\":0,\"queue_capacity\":0,\"queue_high_watermark\":0,\"queued_payload_bytes\":0,\"queued_payload_capacity\":0,\"queued_payload_high_watermark\":0,\"rejected_records\":0,\"write_failures\":0,\"sync_failures\":0,\"last_written_sequence\":null,\"last_durable_sequence\":null,\"last_error\":null}".to_owned();
    };
    let metrics = persistence.producer.metrics();
    let health = persistence.health();
    format!(
        "{{\"schema_version\":1,\"mode\":\"bounded_async\",\"enabled\":{},\"degraded\":{},\"memory_only\":{},\"failure_action\":\"{}\",\"blocks_trading\":{},\"accepted_records\":{},\"written_records\":{},\"abandoned_records\":{},\"records_lag\":{},\"event_time_lag_ns\":{},\"latest_accepted_ts_recv_ns\":{},\"latest_written_ts_recv_ns\":{},\"queue_depth\":{},\"queue_capacity\":{},\"queue_high_watermark\":{},\"queued_payload_bytes\":{},\"queued_payload_capacity\":{},\"queued_payload_high_watermark\":{},\"rejected_records\":{},\"write_failures\":{},\"sync_failures\":{},\"last_written_sequence\":{},\"last_durable_sequence\":{},\"last_error\":{}}}",
        health.enabled,
        health.degraded,
        persistence.memory_only,
        persistence_failure_action_id(persistence.failure_action),
        persistence.blocks_trading(),
        metrics.accepted_records,
        metrics.written_records,
        metrics.abandoned_records,
        health.records_lag,
        metrics.event_time_lag_ns,
        metrics.latest_accepted_ts_recv_ns,
        metrics.latest_written_ts_recv_ns,
        metrics.queue_depth,
        persistence.producer.queue_capacity(),
        metrics.queue_high_watermark,
        metrics.queued_payload_bytes,
        persistence.producer.max_queued_payload_bytes(),
        metrics.queued_payload_high_watermark,
        persistence.rejected_records,
        metrics.write_failures,
        metrics.sync_failures,
        optional_wal_sequence_json(metrics.last_written_sequence),
        optional_wal_sequence_json(metrics.last_synced_sequence),
        optional_str_json(health.last_error.as_deref())
    )
}

pub(crate) fn persistence_failure_action_id(
    action: MarketDataPersistenceFailureAction,
) -> &'static str {
    match action {
        MarketDataPersistenceFailureAction::MarkDegraded => "mark_degraded",
        MarketDataPersistenceFailureAction::StopMarketData => "stop_market_data",
        MarketDataPersistenceFailureAction::StopTrading => "stop_trading",
        MarketDataPersistenceFailureAction::FailProcess => "fail_process",
        MarketDataPersistenceFailureAction::MemoryOnly => "memory_only",
        _ => "unknown",
    }
}

pub(crate) fn optional_wal_sequence_json(
    sequence: Option<of_persist::MarketDataWalSequence>,
) -> String {
    sequence
        .map(|sequence| sequence.0.to_string())
        .unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn format_adapter_descriptor_json(
    descriptor: &AdapterDescriptor,
    active_provider: Option<&ProviderKind>,
) -> String {
    let active = active_provider
        .map(|provider| provider == &descriptor.provider)
        .unwrap_or(false);
    format!(
        "{{\"provider\":\"{}\",\"provider_id\":\"{}\",\"display_name\":\"{}\",\"feature\":{},\"compiled\":{},\"quality\":\"{}\",\"supports_live\":{},\"supports_replay\":{},\"supports_trades\":{},\"supports_order_book\":{},\"supports_level2\":{},\"supports_reconnect\":{},\"supports_gap_recovery\":{},\"supports_backpressure\":{},\"supports_raw_capture\":{},\"supports_fixture_replay\":{},\"supports_stale_detection\":{},\"supports_latency_metrics\":{},\"supports_polling\":{},\"active\":{},\"notes\":\"{}\"}}",
        escape_json(descriptor.provider.id()),
        escape_json(descriptor.provider_id),
        escape_json(descriptor.display_name),
        optional_str_json(descriptor.feature),
        descriptor.compiled,
        escape_json(descriptor.quality.id()),
        descriptor.supports_live,
        descriptor.supports_replay,
        descriptor.supports_trades,
        descriptor.supports_order_book,
        descriptor.supports_level2,
        descriptor.supports_reconnect,
        descriptor.supports_gap_recovery,
        descriptor.supports_backpressure,
        descriptor.supports_raw_capture,
        descriptor.supports_fixture_replay,
        descriptor.supports_stale_detection,
        descriptor.supports_latency_metrics,
        descriptor.supports_polling,
        active,
        escape_json(descriptor.notes)
    )
}

pub(crate) fn format_adapter_inventory_json(active_provider: Option<&ProviderKind>) -> String {
    let items = adapter_descriptors()
        .iter()
        .map(|descriptor| format_adapter_descriptor_json(descriptor, active_provider))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"adapters\":[{}],\"compiled_count\":{},\"total_count\":{}}}",
        items,
        adapter_descriptors()
            .iter()
            .filter(|descriptor| descriptor.compiled)
            .count(),
        adapter_descriptors().len()
    )
}

pub(crate) fn format_adapter_status_json(status: &RuntimeAdapterStatus) -> String {
    let last_error_json = status
        .health
        .last_error
        .as_ref()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .unwrap_or_else(|| "null".to_string());
    let protocol_info_json = status
        .health
        .protocol_info
        .as_ref()
        .map(|s| format!("\"{}\"", escape_json(s)))
        .unwrap_or_else(|| "null".to_string());
    let healthy = status.started
        && status.health.connected
        && !status.health.degraded
        && !status.circuit_breaker_open;
    let subscribed_symbols_json = status
        .operational
        .subscribed_symbols
        .iter()
        .map(|symbol| {
            format!(
                "{{\"venue\":\"{}\",\"symbol\":\"{}\"}}",
                escape_json(&symbol.venue),
                escape_json(&symbol.symbol)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"schema_version\":1,\"provider\":\"{}\",\"provider_id\":\"{}\",\"display_name\":\"{}\",\"feature\":{},\"compiled\":{},\"quality\":\"{}\",\"started\":{},\"connected\":{},\"degraded\":{},\"healthy\":{},\"last_error\":{},\"protocol_info\":{},\"health_seq\":{},\"circuit_breaker_open\":{},\"mode\":\"{}\",\"connection_state\":\"{}\",\"endpoint_redacted\":{},\"app_name\":{},\"reconnect_attempt\":{},\"subscription_count\":{},\"subscribed_symbols\":[{}],\"queue_depth\":{},\"queue_capacity\":{},\"dropped_events\":{},\"gap_count\":{},\"stale\":{},\"raw_capture_enabled\":{},\"raw_capture_depth\":{},\"raw_capture_capacity\":{},\"last_message_age_ms\":{},\"last_market_data_age_ms\":{},\"capabilities\":{}}}",
        escape_json(status.descriptor.provider.id()),
        escape_json(status.descriptor.provider_id),
        escape_json(status.descriptor.display_name),
        optional_str_json(status.descriptor.feature),
        status.descriptor.compiled,
        escape_json(status.descriptor.quality.id()),
        status.started,
        status.health.connected,
        status.health.degraded,
        healthy,
        last_error_json,
        protocol_info_json,
        status.health_seq,
        status.circuit_breaker_open,
        status.operational.mode.id(),
        status.operational.connection_state.id(),
        optional_str_json(status.operational.endpoint_redacted.as_deref()),
        optional_str_json(status.operational.app_name.as_deref()),
        status.operational.reconnect_attempt,
        status.operational.subscription_count,
        subscribed_symbols_json,
        status.operational.queue_depth,
        optional_usize_json(status.operational.queue_capacity),
        status.operational.dropped_events,
        status.operational.gap_count,
        status.operational.stale,
        status.operational.raw_capture_enabled,
        status.operational.raw_capture_depth,
        status.operational.raw_capture_capacity,
        optional_u64_json(status.operational.last_message_age_ms),
        optional_u64_json(status.operational.last_market_data_age_ms),
        format_adapter_descriptor_json(&status.descriptor, Some(&status.descriptor.provider))
    )
}

pub(crate) fn quality_flag_names(bits: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    if bits & DataQualityFlags::STALE_FEED.bits() != 0 {
        names.push("STALE_FEED");
    }
    if bits & DataQualityFlags::SEQUENCE_GAP.bits() != 0 {
        names.push("SEQUENCE_GAP");
    }
    if bits & DataQualityFlags::CLOCK_SKEW.bits() != 0 {
        names.push("CLOCK_SKEW");
    }
    if bits & DataQualityFlags::DEPTH_TRUNCATED.bits() != 0 {
        names.push("DEPTH_TRUNCATED");
    }
    if bits & DataQualityFlags::OUT_OF_ORDER.bits() != 0 {
        names.push("OUT_OF_ORDER");
    }
    if bits & DataQualityFlags::ADAPTER_DEGRADED.bits() != 0 {
        names.push("ADAPTER_DEGRADED");
    }
    names
}

pub(crate) fn quality_flags_detail_json(bits: u32) -> String {
    let names = quality_flag_names(bits);
    if names.is_empty() {
        return "[]".to_string();
    }

    let items = names
        .into_iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

pub(crate) fn sanitize_analytics_config(mut config: AnalyticsConfig) -> AnalyticsConfig {
    config.vpin_volume_bucket = config.vpin_volume_bucket.max(1);
    config.vpin_max_buckets = config.vpin_max_buckets.min(MAX_ANALYTICS_WINDOW_LEN);
    config.kyle_lambda_max_len = config.kyle_lambda_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.cvd_max_len = config.cvd_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.vol_estimator_max_len = config.vol_estimator_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.noise_max_len = config.noise_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.hasbrouck_max_len = config.hasbrouck_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.almgren_chriss_max_len = config.almgren_chriss_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.acd_max_len = config.acd_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.vol_signature_max_len = config.vol_signature_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.agent_max_len = config.agent_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.institutional_max_len = config.institutional_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.resiliency_max_len = config.resiliency_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.spread_decomp_max_len = config.spread_decomp_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.regime_max_len = config.regime_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.event_tracker_max_len = config.event_tracker_max_len.min(MAX_EVENT_TRACKER_LEN);
    config.spread_tracker_max_len = config.spread_tracker_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config.default_max_len = config.default_max_len.min(MAX_ANALYTICS_WINDOW_LEN);
    config
}
