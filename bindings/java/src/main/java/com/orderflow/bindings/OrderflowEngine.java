package com.orderflow.bindings;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;

/**
 * High-level Java wrapper around the Orderflow C ABI.
 *
 * <p>This is the primary JVM entry point for runtime lifecycle, symbol stream subscription,
 * external data ingestion, and snapshot retrieval.
 *
 * <p>Lifecycle contract:
 * <ol>
 *   <li>Create instance using configuration and optional native library path.</li>
 *   <li>Call {@link #start()} (or use inside application startup flow).</li>
 *   <li>Subscribe symbols, then call {@link #pollOnce(int)} and/or ingest events.</li>
 *   <li>Call {@link #close()} (or use try-with-resources) to release native resources.</li>
 * </ol>
 */
public final class OrderflowEngine implements AutoCloseable {
    private final OrderflowNative nativeLib;
    private Pointer engine;
    private final List<Pointer> subscriptions = new ArrayList<>();
    private final List<OfEventCallback> callbacks = new ArrayList<>();

    /**
     * Creates an engine using config and an optional explicit native library path.
     *
     * @param nativePath library path, or null/blank for default lookup
     * @param config runtime configuration values
     * @throws OrderflowException if native engine creation fails
     */
    public OrderflowEngine(String nativePath, EngineConfig config) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        this.nativeLib = OrderflowNative.load(libPath);

        OfEngineConfig cfg = new OfEngineConfig();
        cfg.instance_id = config.instanceId;
        cfg.config_path = config.configPath;
        cfg.log_level = config.logLevel;
        cfg.enable_persistence = (byte) (config.enablePersistence ? 1 : 0);
        cfg.audit_max_bytes = config.auditMaxBytes;
        cfg.audit_max_files = config.auditMaxFiles;
        cfg.audit_redact_tokens_csv = config.auditRedactTokensCsv;
        cfg.data_retention_max_bytes = config.dataRetentionMaxBytes;
        cfg.data_retention_max_age_secs = config.dataRetentionMaxAgeSecs;
        cfg.write();

        PointerByReference outEngine = new PointerByReference();
        int rc = nativeLib.of_engine_create(cfg, outEngine);
        check(rc, "of_engine_create");
        this.engine = outEngine.getValue();
    }

    /**
     * Returns native ABI version.
     *
     * @return ABI version as integer encoding
     */
    public int apiVersion() {
        return nativeLib.of_api_version();
    }

    /**
     * Returns native build info string.
     *
     * @return build descriptor from the native runtime
     */
    public String buildInfo() {
        return nativeLib.of_build_info();
    }

    /**
     * Returns market-data adapter inventory JSON for the native build.
     *
     * @param nativePath library path, or null/blank for default lookup
     * @return JSON payload with known providers, feature gates, and capabilities
     */
    public static String adapterInventory(String nativePath) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        OrderflowNative nativeLib = OrderflowNative.load(libPath);
        return allocatedJson(nativeLib, null, "of_get_adapter_inventory_json");
    }

    /**
     * Returns built-in signal descriptor inventory JSON for the native build.
     *
     * @param nativePath library path, or null/blank for default lookup
     * @return JSON payload with built-in signal metadata and requirements
     */
    public static String signalDescriptors(String nativePath) {
        String libPath = nativePath == null || nativePath.isBlank() ? defaultLibraryPath() : nativePath;
        OrderflowNative nativeLib = OrderflowNative.load(libPath);
        return allocatedJson(nativeLib, null, "of_get_signal_descriptors_json");
    }

    /**
     * Validates registry configuration for a built-in signal.
     *
     * @param config signal identifier and typed descriptor parameters
     * @param nativePath library path, or null/blank for default lookup
     * @return parsed registry validation result
     */
    public static SignalConfigValidation validateSignalConfig(
            SignalConfig config, String nativePath) {
        String libPath = nativePath == null || nativePath.isBlank()
            ? defaultLibraryPath() : nativePath;
        OrderflowNative nativeLib = OrderflowNative.load(libPath);
        OfSignalConfigParameter[] parameters = toNativeSignalParameters(config);
        PointerByReference out = new PointerByReference();
        IntByReference outLen = new IntByReference(0);
        int rc = nativeLib.of_validate_signal_config_json(
            config.signalId, parameters, config.parameters.size(), out, outLen);
        check(rc, "of_validate_signal_config_json");
        String json = takeAllocatedJson(nativeLib, out);
        return new SignalConfigValidation(
            NativeSignalJson.nullableString(json, "signal_id"),
            NativeSignalJson.booleanValue(json, "valid"),
            NativeSignalJson.nullableString(json, "error"),
            json);
    }

    /**
     * Constructs a built-in signal and validates it over ordered observations.
     *
     * @param config signal identifier and typed descriptor parameters
     * @param events ordered analytics observations
     * @param validationConfig markout and timestamp policy
     * @param nativePath library path, or null/blank for default lookup
     * @return parsed validation summary retaining the complete native JSON
     */
    public static SignalValidationReport validateSignalReplay(
            SignalConfig config,
            List<SignalValidationEvent> events,
            SignalValidationConfig validationConfig,
            String nativePath) {
        if (events == null || validationConfig == null) {
            throw new IllegalArgumentException("events and validationConfig are required");
        }
        String libPath = nativePath == null || nativePath.isBlank()
            ? defaultLibraryPath() : nativePath;
        OrderflowNative nativeLib = OrderflowNative.load(libPath);
        OfSignalConfigParameter[] parameters = toNativeSignalParameters(config);
        OfSignalValidationEvent[] nativeEvents = toNativeSignalEvents(events);
        OfSignalValidationConfig nativeConfig = new OfSignalValidationConfig();
        nativeConfig.markout_horizon_events = (int) validationConfig.markoutHorizonEvents;
        nativeConfig.flat_price_threshold = validationConfig.flatPriceThreshold;
        nativeConfig.min_confidence_bps = (short) validationConfig.minConfidenceBps;
        nativeConfig.store_samples = (byte) (validationConfig.storeSamples ? 1 : 0);
        nativeConfig.check_monotonic_timestamps =
            (byte) (validationConfig.checkMonotonicTimestamps ? 1 : 0);
        nativeConfig.write();

        PointerByReference out = new PointerByReference();
        IntByReference outLen = new IntByReference(0);
        int rc = nativeLib.of_validate_signal_replay_json(
            config.signalId,
            parameters,
            config.parameters.size(),
            nativeEvents,
            events.size(),
            nativeConfig,
            out,
            outLen);
        check(rc, "of_validate_signal_replay_json");
        return SignalValidationReport.parse(takeAllocatedJson(nativeLib, out));
    }

    /**
     * Starts engine processing.
     *
     * @throws OrderflowStateException if the runtime cannot start from current state
     */
    public void start() {
        requireEngine();
        check(nativeLib.of_engine_start(engine), "of_engine_start");
    }

    /**
     * Stops engine processing.
     *
     * @throws OrderflowStateException if stop fails while runtime handle exists
     */
    public void stop() {
        if (engine != null) {
            check(nativeLib.of_engine_stop(engine), "of_engine_stop");
        }
    }

    /**
     * Subscribes a symbol stream without callback listener.
     *
     * @param symbol target venue/instrument/depth descriptor
     * @param streamKind stream identifier from {@link StreamKind}
     */
    public void subscribe(Symbol symbol, int streamKind) {
        subscribe(symbol, streamKind, null);
    }

    /**
     * Subscribes a symbol stream with optional callback listener.
     *
     * @param symbol target venue/instrument/depth descriptor
     * @param streamKind stream identifier from {@link StreamKind}
     * @param listener nullable callback for event delivery
     */
    public void subscribe(Symbol symbol, int streamKind, EventListener listener) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();

        PointerByReference outSub = new PointerByReference();
        OfEventCallback cb = null;
        if (listener != null) {
            cb = (evPtr, userData) -> {
                OfEvent ev = new OfEvent(evPtr);
                String payload = "{}";
                if (ev.payload != null && ev.payload_len > 0) {
                    payload = new String(
                            ev.payload.getByteArray(0, ev.payload_len),
                            StandardCharsets.UTF_8);
                }
                listener.onEvent(new OrderflowEvent(
                        ev.ts_exchange_ns,
                        ev.ts_recv_ns,
                        ev.kind,
                        ev.schema_id,
                        ev.quality_flags,
                        payload));
            };
            callbacks.add(cb);
        }

        int rc = nativeLib.of_subscribe(engine, sym, streamKind, cb, null, outSub);
        check(rc, "of_subscribe");
        subscriptions.add(outSub.getValue());
    }

    /**
     * Polls adapter/runtime once and dispatches callback events.
     *
     * @param qualityFlags quality context bits (typically {@link DataQualityFlags#NONE})
     */
    public void pollOnce(int qualityFlags) {
        requireEngine();
        check(nativeLib.of_engine_poll_once(engine, qualityFlags), "of_engine_poll_once");
    }

    /**
     * Unsubscribes all streams for a symbol.
     *
     * @param symbol symbol descriptor to remove from active subscriptions
     */
    public void unsubscribe(Symbol symbol) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();
        check(nativeLib.of_unsubscribe_symbol(engine, sym), "of_unsubscribe_symbol");
    }

    /**
     * Resets per-symbol analytics session state.
     *
     * @param symbol symbol whose session/profile state should be cleared
     */
    public void resetSymbolSession(Symbol symbol) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();
        check(nativeLib.of_reset_symbol_session(engine, sym), "of_reset_symbol_session");
    }

    /**
     * Configures stale/sequence supervision for external ingest flow.
     *
     * @param staleAfterMs stale threshold in milliseconds
     * @param enforceSequence whether sequence checks should be enforced
     */
    public void configureExternalFeed(long staleAfterMs, boolean enforceSequence) {
        requireEngine();
        OfExternalFeedPolicy policy = new OfExternalFeedPolicy();
        policy.stale_after_ms = staleAfterMs;
        policy.enforce_sequence = (byte) (enforceSequence ? 1 : 0);
        policy.write();
        check(nativeLib.of_configure_external_feed(engine, policy), "of_configure_external_feed");
    }

    /**
     * Marks external feed reconnecting/degraded state.
     *
     * @param reconnecting true while feed is reconnecting/degraded
     */
    public void setExternalReconnecting(boolean reconnecting) {
        requireEngine();
        check(
                nativeLib.of_external_set_reconnecting(engine, (byte) (reconnecting ? 1 : 0)),
                "of_external_set_reconnecting");
    }

    /**
     * Sets the tickbar aggregation interval for new per-symbol accumulators.
     *
     * <p>A positive {@code intervalNs} enables tickbar aggregation at the
     * given interval for symbols whose accumulators are created after this
     * call. Zero or negative values disable tickbar aggregation for future
     * accumulators. Existing accumulators are not affected.
     *
     * <p>Requires the native library to be built with the {@code tickbar} feature.
     *
     * @param intervalNs aggregation interval in nanoseconds (0 or negative to disable)
     */
    public void setTickbarInterval(long intervalNs) {
        requireEngine();
        check(
                nativeLib.of_engine_set_tickbar_interval(engine, intervalNs),
                "of_engine_set_tickbar_interval");
    }

    /** Re-evaluates external-feed health without ingesting new events. */
    public void externalHealthTick() {
        requireEngine();
        check(nativeLib.of_external_health_tick(engine), "of_external_health_tick");
    }

    /**
     * Convenience overload for ingesting one trade with default metadata.
     *
     * @param symbol target symbol
     * @param price integerized trade price
     * @param size trade size/quantity
     * @param aggressorSide aggressor side from {@link Side}
     */
    public void ingestTrade(Symbol symbol, long price, long size, int aggressorSide) {
        ingestTrade(symbol, price, size, aggressorSide, 0L, 0L, 0L, DataQualityFlags.NONE);
    }

    /**
     * Ingests one external trade event into runtime processing.
     *
     * @param symbol target symbol
     * @param price integerized trade price
     * @param size trade size/quantity
     * @param aggressorSide aggressor side from {@link Side}
     * @param sequence external feed sequence number
     * @param tsExchangeNs exchange timestamp in nanoseconds
     * @param tsRecvNs receive timestamp in nanoseconds
     * @param qualityFlags quality context bits from {@link DataQualityFlags}
     */
    public void ingestTrade(
            Symbol symbol,
            long price,
            long size,
            int aggressorSide,
            long sequence,
            long tsExchangeNs,
            long tsRecvNs,
            int qualityFlags) {
        requireEngine();
        OfTrade trade = new OfTrade();
        trade.symbol = toNativeSymbol(symbol);
        trade.price = price;
        trade.size = size;
        trade.aggressor_side = aggressorSide;
        trade.sequence = sequence;
        trade.ts_exchange_ns = tsExchangeNs;
        trade.ts_recv_ns = tsRecvNs;
        trade.write();
        check(nativeLib.of_ingest_trade(engine, trade, qualityFlags), "of_ingest_trade");
    }

    /**
     * Convenience overload for ingesting one book update with default metadata.
     *
     * @param symbol target symbol
     * @param side side from {@link Side}
     * @param level depth level index
     * @param price integerized level price
     * @param size level size/quantity
     */
    public void ingestBook(Symbol symbol, int side, int level, long price, long size) {
        ingestBook(
                symbol,
                side,
                level,
                price,
                size,
                BookAction.UPSERT,
                0L,
                0L,
                0L,
                DataQualityFlags.NONE);
    }

    /**
     * Ingests one external book event into runtime processing.
     *
     * @param symbol target symbol
     * @param side side from {@link Side}
     * @param level depth level index
     * @param price integerized level price
     * @param size level quantity
     * @param action action from {@link BookAction}
     * @param sequence external feed sequence number
     * @param tsExchangeNs exchange timestamp in nanoseconds
     * @param tsRecvNs receive timestamp in nanoseconds
     * @param qualityFlags quality context bits from {@link DataQualityFlags}
     */
    public void ingestBook(
            Symbol symbol,
            int side,
            int level,
            long price,
            long size,
            int action,
            long sequence,
            long tsExchangeNs,
            long tsRecvNs,
            int qualityFlags) {
        requireEngine();
        OfBook book = new OfBook();
        book.symbol = toNativeSymbol(symbol);
        book.side = side;
        book.level = (short) level;
        book.price = price;
        book.size = size;
        book.action = action;
        book.sequence = sequence;
        book.ts_exchange_ns = tsExchangeNs;
        book.ts_recv_ns = tsRecvNs;
        book.write();
        check(nativeLib.of_ingest_book(engine, book, qualityFlags), "of_ingest_book");
    }

    /**
     * Returns current book snapshot as JSON string.
     *
     * @param symbol target symbol
     * @return JSON payload with venue, symbol, bids, asks, last_sequence, and timestamps
     */
    public String bookSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.BOOK);
    }

    /**
     * Returns current book analytics snapshot (spread, depth, imbalance, microprice) as JSON string.
     *
     * @param symbol target symbol
     * @return JSON payload with computed book metrics
     */
    public String bookAnalyticsSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.BOOK_ANALYTICS);
    }

    /**
     * Returns current analytics snapshot as JSON string.
     *
     * @param symbol target symbol
     * @return JSON snapshot payload
     */
    public String analyticsSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.ANALYTICS);
    }

    /**
     * Returns current derived analytics snapshot as JSON string.
     *
     * @param symbol target symbol
     * @return JSON payload with session volume, trade count, vwap, average trade size, and imbalance_bps
     */
    public String derivedAnalyticsSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.DERIVED_ANALYTICS);
    }

    /**
     * Returns current session candle snapshot as JSON string.
     *
     * @param symbol target symbol
     * @return JSON payload with open, high, low, close, trade_count, and first/last exchange timestamps
     */
    public String sessionCandleSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.SESSION_CANDLE);
    }

    /**
     * Returns rolling interval candle snapshot as JSON string.
     *
     * @param symbol target symbol
     * @param windowNs rolling interval width in nanoseconds
     * @return JSON payload with interval open/high/low/close, trade_count, total_volume, vwap, and timestamps
     */
    public String intervalCandleSnapshot(Symbol symbol, long windowNs) {
        return snapshot(symbol, SnapshotKind.INTERVAL_CANDLE, windowNs);
    }

    /**
     * Returns current signal snapshot as JSON string.
     *
     * @param symbol target symbol
     * @return JSON snapshot payload
     */
    public String signalSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.SIGNAL);
    }

    /**
     * Returns latest signal explanation as JSON string.
     *
     * @param symbol target symbol
     * @return JSON explanation payload, or empty object when no signal has evaluated yet
     */
    public String signalExplanation(Symbol symbol) {
        requireEngine();
        return allocatedSignalExplanationJson(nativeLib, engine, toNativeSymbol(symbol));
    }

    /**
     * Returns signal metrics as JSON string.
     *
     * @return JSON payload with signal state counts, confidence, and explanation coverage
     */
    public String signalMetrics() {
        requireEngine();
        return allocatedJson(nativeLib, engine, "of_get_signal_metrics_json");
    }

    /**
     * Returns completed bar series JSON array for a symbol.
     *
     * <p>Requires the native library to be built with the {@code tickbar} feature.
     * Returns {@code []} when tickbar aggregation is not configured for the symbol.
     *
     * @param symbol target symbol
     * @return JSON array of bar objects, each with timestamp_ns, open, high, low, close, volume, tick_count, vwap
     */
    public String barSeries(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.BAR_SERIES);
    }

    /**
     * Returns runtime metrics as JSON string.
     *
     * @return runtime metrics payload
     */
    public String metricsJson() {
        requireEngine();
        return allocatedJson(nativeLib, engine, "of_get_metrics_json");
    }

    /**
     * Returns market-data adapter inventory JSON for the native build.
     *
     * @return JSON payload with known providers, feature gates, and capabilities
     */
    public String adapterInventory() {
        return allocatedJson(nativeLib, null, "of_get_adapter_inventory_json");
    }

    /**
     * Returns active adapter descriptor and health status as JSON.
     *
     * @return JSON payload for the engine's configured adapter
     */
    public String adapterStatus() {
        requireEngine();
        return allocatedJson(nativeLib, engine, "of_get_active_adapter_status_json");
    }

    /**
     * Returns built-in signal descriptor inventory JSON for this native library.
     *
     * @return JSON payload with built-in signal metadata and requirements
     */
    public String signalDescriptors() {
        return allocatedJson(nativeLib, null, "of_get_signal_descriptors_json");
    }

    /**
     * Unsubscribes active subscriptions and destroys native engine handle.
     *
     * <p>Safe to call multiple times; subsequent calls are no-ops.
     */
    @Override
    public void close() {
        if (engine == null) {
            return;
        }

        for (Pointer sub : subscriptions) {
            if (sub != null) {
                nativeLib.of_unsubscribe(sub);
            }
        }
        subscriptions.clear();
        callbacks.clear();

        nativeLib.of_engine_destroy(engine);
        engine = null;
    }

    private String snapshot(Symbol symbol, SnapshotKind kind) {
        return snapshot(symbol, kind, 0L);
    }

    private String snapshot(Symbol symbol, SnapshotKind kind, long windowNs) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();

        int capacity = 4096;
        for (int attempt = 0; attempt < 3; attempt++) {
            Memory buffer = new Memory(capacity);
            IntByReference length = new IntByReference(capacity);

            int rc;
            switch (kind) {
                case BOOK -> rc = nativeLib.of_get_book_snapshot(engine, sym, buffer, length);
                case BOOK_ANALYTICS -> rc = nativeLib.of_get_book_analytics_snapshot(engine, sym, buffer, length);
                case ANALYTICS -> rc = nativeLib.of_get_analytics_snapshot(engine, sym, buffer, length);
                case DERIVED_ANALYTICS -> rc = nativeLib.of_get_derived_analytics_snapshot(engine, sym, buffer, length);
                case SESSION_CANDLE -> rc = nativeLib.of_get_session_candle_snapshot(engine, sym, buffer, length);
                case INTERVAL_CANDLE -> rc = nativeLib.of_get_interval_candle_snapshot(engine, sym, windowNs, buffer, length);
                case SIGNAL -> rc = nativeLib.of_get_signal_snapshot(engine, sym, buffer, length);
                case BAR_SERIES -> rc = nativeLib.of_get_bar_series(engine, sym, buffer, length);
                case MID_PRICE -> rc = nativeLib.of_get_mid_price(engine, sym, buffer, length);
                case EFFECTIVE_SPREAD -> rc = nativeLib.of_get_effective_spread_bps(engine, sym, buffer, length);
                case RESILIENCY -> rc = nativeLib.of_get_resiliency_snapshot(engine, sym, buffer, length);
                case VPIN -> rc = nativeLib.of_get_vpin_snapshot(engine, sym, buffer, length);
                case KYLE_LAMBDA -> rc = nativeLib.of_get_kyle_lambda_snapshot(engine, sym, buffer, length);
                case AMIHUD -> rc = nativeLib.of_get_amihud_snapshot(engine, sym, buffer, length);
                case CVD_ENHANCEMENT -> rc = nativeLib.of_get_cvd_enhancement_snapshot(engine, sym, buffer, length);
                case PATTERN -> rc = nativeLib.of_get_pattern_snapshot(engine, sym, buffer, length);
                case VOLATILITY -> rc = nativeLib.of_get_volatility_snapshot(engine, sym, buffer, length);
                case NOISE -> rc = nativeLib.of_get_noise_snapshot(engine, sym, buffer, length);
                case HASBROUCK -> rc = nativeLib.of_get_hasbrouck_snapshot(engine, sym, buffer, length);
                case ALMGREN_CHRISS -> rc = nativeLib.of_get_almgren_chriss_snapshot(engine, sym, buffer, length);
                case SPREAD_DECOMP -> rc = nativeLib.of_get_spread_decomp_snapshot(engine, sym, buffer, length);
                case ACD -> rc = nativeLib.of_get_acd_snapshot(engine, sym, buffer, length);
                case REGIME -> rc = nativeLib.of_get_regime_snapshot(engine, sym, buffer, length);
                case KINETIC_ENERGY -> rc = nativeLib.of_get_kinetic_energy_snapshot(engine, sym, buffer, length);
                case DARK_POOL -> rc = nativeLib.of_get_dark_pool_snapshot(engine, sym, buffer, length);
                case OPTIONS_FLOW -> rc = nativeLib.of_get_options_flow_snapshot(engine, sym, buffer, length);
                case FUTURES -> rc = nativeLib.of_get_futures_snapshot(engine, sym, buffer, length);
                case VOL_SIGNATURE -> rc = nativeLib.of_get_vol_signature_snapshot(engine, sym, buffer, length);
                case AGENT_TYPE -> rc = nativeLib.of_get_agent_type_snapshot(engine, sym, buffer, length);
                case DARK_LIT_CORRELATION -> rc = nativeLib.of_get_dark_lit_correlation_snapshot(engine, sym, buffer, length);
                case INSTITUTIONAL_FLOW -> rc = nativeLib.of_get_institutional_flow_snapshot(engine, sym, buffer, length);
                case OI_ANALYSIS -> rc = nativeLib.of_get_oi_analysis_snapshot(engine, sym, buffer, length);
                default -> throw new OrderflowException("unknown snapshot kind");
            }

            if (rc == 0) {
                int outLen = length.getValue();
                if (outLen <= 0) {
                    return "{}";
                }
                return new String(buffer.getByteArray(0, outLen), StandardCharsets.UTF_8);
            }

            int required = length.getValue();
            if (rc != 1 || required <= capacity) {
                check(rc, "snapshot");
            }
            capacity = required;
        }

        throw new OrderflowArgException("snapshot failed with OF_ERR_INVALID_ARG");
    }

    private static OfSymbol toNativeSymbol(Symbol symbol) {
        OfSymbol s = new OfSymbol();
        s.venue = symbol.venue;
        s.symbol = symbol.symbol;
        s.depth_levels = (short) symbol.depthLevels;
        return s;
    }

    private static void check(int rc, String fn) {
        if (rc == 0) {
            return;
        }
        if (rc == 1) {
            throw new OrderflowArgException(fn + " failed with OF_ERR_INVALID_ARG");
        }
        if (rc == 2) {
            throw new OrderflowStateException(fn + " failed with OF_ERR_STATE");
        }
        throw new OrderflowException(fn + " failed with error code " + rc);
    }

    private static String allocatedJson(OrderflowNative nativeLib, Pointer engine, String fn) {
        PointerByReference out = new PointerByReference();
        IntByReference outLen = new IntByReference(0);
        int rc;
        if ("of_get_metrics_json".equals(fn)) {
            rc = nativeLib.of_get_metrics_json(engine, out, outLen);
        } else if ("of_get_adapter_inventory_json".equals(fn)) {
            rc = nativeLib.of_get_adapter_inventory_json(out, outLen);
        } else if ("of_get_active_adapter_status_json".equals(fn)) {
            rc = nativeLib.of_get_active_adapter_status_json(engine, out, outLen);
        } else if ("of_get_signal_descriptors_json".equals(fn)) {
            rc = nativeLib.of_get_signal_descriptors_json(out, outLen);
        } else if ("of_get_signal_metrics_json".equals(fn)) {
            rc = nativeLib.of_get_signal_metrics_json(engine, out, outLen);
        } else {
            throw new OrderflowException("unknown allocated JSON function: " + fn);
        }
        check(rc, fn);
        Pointer p = out.getValue();
        if (p == null) {
            return "{}";
        }
        try {
            return p.getString(0, StandardCharsets.UTF_8.name());
        } finally {
            nativeLib.of_string_free(p);
        }
    }

    private static String takeAllocatedJson(
            OrderflowNative nativeLib, PointerByReference out) {
        Pointer pointer = out.getValue();
        if (pointer == null) {
            return "{}";
        }
        try {
            return pointer.getString(0, StandardCharsets.UTF_8.name());
        } finally {
            nativeLib.of_string_free(pointer);
        }
    }

    private static OfSignalConfigParameter[] toNativeSignalParameters(SignalConfig config) {
        if (config == null) {
            throw new IllegalArgumentException("config is required");
        }
        if (config.parameters.isEmpty()) {
            return null;
        }
        OfSignalConfigParameter[] nativeParameters =
            (OfSignalConfigParameter[]) new OfSignalConfigParameter()
                .toArray(config.parameters.size());
        for (int index = 0; index < config.parameters.size(); index++) {
            SignalConfigParameter parameter = config.parameters.get(index);
            OfSignalConfigParameter nativeParameter = nativeParameters[index];
            nativeParameter.name = parameter.name;
            nativeParameter.kind = parameter.kind;
            nativeParameter.integer_value = parameter.integerValue;
            nativeParameter.float_value = parameter.floatValue;
            nativeParameter.boolean_value = (byte) (parameter.booleanValue ? 1 : 0);
            nativeParameter.text_value = parameter.textValue;
            nativeParameter.write();
        }
        return nativeParameters;
    }

    private static OfSignalValidationEvent[] toNativeSignalEvents(
            List<SignalValidationEvent> events) {
        if (events.isEmpty()) {
            return null;
        }
        OfSignalValidationEvent[] nativeEvents =
            (OfSignalValidationEvent[]) new OfSignalValidationEvent().toArray(events.size());
        for (int index = 0; index < events.size(); index++) {
            SignalValidationEvent event = events.get(index);
            if (event == null) {
                throw new IllegalArgumentException("validation events must not contain null");
            }
            OfSignalValidationEvent nativeEvent = nativeEvents[index];
            nativeEvent.delta = event.delta;
            nativeEvent.cumulative_delta = event.cumulativeDelta;
            nativeEvent.buy_volume = event.buyVolume;
            nativeEvent.sell_volume = event.sellVolume;
            nativeEvent.last_price = event.lastPrice;
            nativeEvent.point_of_control = event.pointOfControl;
            nativeEvent.value_area_low = event.valueAreaLow;
            nativeEvent.value_area_high = event.valueAreaHigh;
            nativeEvent.ts_exchange_ns = event.tsExchangeNs == null ? 0 : event.tsExchangeNs;
            nativeEvent.has_ts_exchange_ns = (byte) (event.tsExchangeNs == null ? 0 : 1);
            nativeEvent.write();
        }
        return nativeEvents;
    }

    private static String allocatedSignalExplanationJson(
            OrderflowNative nativeLib,
            Pointer engine,
            OfSymbol symbol) {
        PointerByReference out = new PointerByReference();
        IntByReference outLen = new IntByReference(0);
        int rc = nativeLib.of_get_signal_explanation_json(engine, symbol, out, outLen);
        check(rc, "of_get_signal_explanation_json");
        Pointer p = out.getValue();
        if (p == null) {
            return "{}";
        }
        try {
            return p.getString(0, StandardCharsets.UTF_8.name());
        } finally {
            nativeLib.of_string_free(p);
        }
    }

    private void requireEngine() {
        if (engine == null) {
            throw new OrderflowStateException("engine is closed");
        }
    }

    private static String defaultLibraryPath() {
        String env = System.getenv("ORDERFLOW_LIBRARY_PATH");
        if (env != null && !env.isBlank()) {
            return env;
        }
        String mapped = System.mapLibraryName("of_ffi_c");
        return "target/debug/" + mapped;
    }

    private String parameterizedQuery(Symbol symbol, QueryKind kind, long param) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();

        int capacity = 4096;
        for (int attempt = 0; attempt < 3; attempt++) {
            Memory buffer = new Memory(capacity);
            IntByReference length = new IntByReference(capacity);

            int rc;
            switch (kind) {
                case WEIGHTED_AVERAGE_PRICE -> rc = nativeLib.of_compute_weighted_average_price(engine, sym, param, buffer, length);
                case DEPTH_SLOPE -> rc = nativeLib.of_compute_depth_slope(engine, sym, (int) param, buffer, length);
                case HALF_SPREAD_COST -> rc = nativeLib.of_get_half_spread_cost_bps(engine, sym, (int) param, buffer, length);
                case REALISED_SPREAD -> rc = nativeLib.of_get_realised_spread_bps(engine, sym, (int) param, buffer, length);
                case BOOK_EVENT_ANALYTICS -> rc = nativeLib.of_get_book_event_analytics(engine, sym, param, buffer, length);
                default -> throw new OrderflowException("unknown query kind");
            }

            if (rc == 0) {
                int outLen = length.getValue();
                if (outLen <= 0) {
                    return "{}";
                }
                return new String(buffer.getByteArray(0, outLen), StandardCharsets.UTF_8);
            }

            int required = length.getValue();
            if (rc != 1 || required <= capacity) {
                check(rc, "parameterizedQuery");
            }
            capacity = required;
        }

        check(1, "parameterizedQuery");
        return "{}";
    }

    // Convenience methods for new T0 analytics

    /** Returns mid price JSON. */
    public String midPrice(Symbol symbol) { return snapshot(symbol, SnapshotKind.MID_PRICE); }

    /** Returns last effective spread in bps JSON. */
    public String effectiveSpreadBps(Symbol symbol) { return snapshot(symbol, SnapshotKind.EFFECTIVE_SPREAD); }

    /** Returns average half-spread cost in bps over `window` trades. */
    public String halfSpreadCostBps(Symbol symbol, int window) {
        return parameterizedQuery(symbol, QueryKind.HALF_SPREAD_COST, window);
    }

    /** Returns realised spread in bps for trade `holdTicks` ago. */
    public String realisedSpreadBps(Symbol symbol, int holdTicks) {
        return parameterizedQuery(symbol, QueryKind.REALISED_SPREAD, holdTicks);
    }

    /** Returns book-event analytics snapshot JSON over `windowNs` window. */
    public String bookEventAnalytics(Symbol symbol, long windowNs) {
        return parameterizedQuery(symbol, QueryKind.BOOK_EVENT_ANALYTICS, windowNs);
    }

    /** Returns resiliency snapshot JSON. */
    public String resiliencySnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.RESILIENCY); }
    /** Returns VPIN snapshot JSON. */
    public String vpinSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.VPIN); }
    /** Returns Kyle's Lambda snapshot JSON. */
    public String kyleLambdaSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.KYLE_LAMBDA); }
    /** Returns Amihud illiquidity snapshot JSON. */
    public String amihudSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.AMIHUD); }
    /** Returns CVD enhancement snapshot JSON. */
    public String cvdEnhancementSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.CVD_ENHANCEMENT); }

    /** Returns pattern detection snapshot JSON. */
    public String patternSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.PATTERN); }

    /** Returns realised volatility estimator snapshot JSON. */
    public String volatilitySnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.VOLATILITY); }
    /** Returns microstructure noise snapshot JSON. */
    public String noiseSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.NOISE); }
    /** Returns Hasbrouck impact snapshot JSON. */
    public String hasbrouckSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.HASBROUCK); }
    /** Returns Almgren-Chriss impact snapshot JSON. */
    public String almgrenChrissSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.ALMGREN_CHRISS); }
    /** Returns spread decomposition snapshot JSON. */
    public String spreadDecompSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.SPREAD_DECOMP); }
    /** Returns ACD duration-model snapshot JSON. */
    public String acdSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.ACD); }
    /** Returns regime detection snapshot JSON. */
    public String regimeSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.REGIME); }

    /** Returns order-book kinetic-energy snapshot JSON. */
    public String kineticEnergySnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.KINETIC_ENERGY); }
    /** Returns dark-pool analytics snapshot JSON. */
    public String darkPoolSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.DARK_POOL); }
    /** Returns options-flow analytics snapshot JSON. */
    public String optionsFlowSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.OPTIONS_FLOW); }
    /** Returns futures basis and roll snapshot JSON. */
    public String futuresSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.FUTURES); }

    /** Returns volatility-signature snapshot JSON. */
    public String volSignatureSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.VOL_SIGNATURE); }
    /** Returns agent-type identification snapshot JSON. */
    public String agentTypeSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.AGENT_TYPE); }
    /** Returns dark-lit correlation snapshot JSON. */
    public String darkLitCorrelationSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.DARK_LIT_CORRELATION); }
    /** Returns institutional-flow snapshot JSON. */
    public String institutionalFlowSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.INSTITUTIONAL_FLOW); }
    /** Returns open-interest analysis snapshot JSON. */
    public String oiAnalysisSnapshot(Symbol symbol) { return snapshot(symbol, SnapshotKind.OI_ANALYSIS); }

    /** Computes LOB feature snapshot JSON from book state and supplied flow metrics. */
    public String lobFeatures(Symbol symbol, double tradeImbalance, double cancelRate, double arrivalRate) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        int capacity = 8192;
        Memory buf = new Memory(capacity);
        IntByReference len = new IntByReference(capacity);
        int rc = nativeLib.of_compute_lob_features(engine, sym, tradeImbalance, cancelRate, arrivalRate, buf, len);
        if (rc == 0) { return buf.getString(0); }
        check(rc, "lobFeatures");
        throw new OrderflowArgException("lobFeatures failed");
    }

    /** Applies a native analytics configuration pointer, or null to reset defaults. */
    public void setAnalyticsConfig(Pointer config) {
        requireEngine();
        int rc = nativeLib.of_engine_set_analytics_config(engine, config);
        check(rc, "setAnalyticsConfig");
    }

    private enum SnapshotKind {
        BOOK,
        BOOK_ANALYTICS,
        ANALYTICS,
        DERIVED_ANALYTICS,
        SESSION_CANDLE,
        INTERVAL_CANDLE,
        SIGNAL,
        BAR_SERIES,
        MID_PRICE,
        EFFECTIVE_SPREAD,
        RESILIENCY,
        VPIN,
        KYLE_LAMBDA,
        AMIHUD,
        CVD_ENHANCEMENT,
        PATTERN,
        VOLATILITY,
        NOISE,
        HASBROUCK,
        ALMGREN_CHRISS,
        SPREAD_DECOMP,
        ACD,
        REGIME,
        KINETIC_ENERGY,
        DARK_POOL,
        OPTIONS_FLOW,
        FUTURES,
        VOL_SIGNATURE,
        AGENT_TYPE,
        DARK_LIT_CORRELATION,
        INSTITUTIONAL_FLOW,
        OI_ANALYSIS,
    }

    private enum QueryKind {
        WEIGHTED_AVERAGE_PRICE,
        DEPTH_SLOPE,
        HALF_SPREAD_COST,
        REALISED_SPREAD,
        BOOK_EVENT_ANALYTICS,
    }
}
