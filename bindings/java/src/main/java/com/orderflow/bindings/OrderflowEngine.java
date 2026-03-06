package com.orderflow.bindings;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

import com.sun.jna.Memory;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.PointerByReference;

/** High-level Java wrapper around the Orderflow C ABI. */
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

    /** Returns native ABI version. */
    public int apiVersion() {
        return nativeLib.of_api_version();
    }

    /** Returns native build info string. */
    public String buildInfo() {
        return nativeLib.of_build_info();
    }

    /** Starts engine processing. */
    public void start() {
        requireEngine();
        check(nativeLib.of_engine_start(engine), "of_engine_start");
    }

    /** Stops engine processing. */
    public void stop() {
        if (engine != null) {
            check(nativeLib.of_engine_stop(engine), "of_engine_stop");
        }
    }

    /** Subscribes symbol stream without callback listener. */
    public void subscribe(Symbol symbol, int streamKind) {
        subscribe(symbol, streamKind, null);
    }

    /** Subscribes symbol stream with optional callback listener. */
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

    /** Polls adapter once and dispatches callback events. */
    public void pollOnce(int qualityFlags) {
        requireEngine();
        check(nativeLib.of_engine_poll_once(engine, qualityFlags), "of_engine_poll_once");
    }

    /** Unsubscribes all streams for symbol. */
    public void unsubscribe(Symbol symbol) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();
        check(nativeLib.of_unsubscribe_symbol(engine, sym), "of_unsubscribe_symbol");
    }

    /** Resets per-symbol analytics session state. */
    public void resetSymbolSession(Symbol symbol) {
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();
        check(nativeLib.of_reset_symbol_session(engine, sym), "of_reset_symbol_session");
    }

    /** Configures stale/sequence supervision for external ingest flow. */
    public void configureExternalFeed(long staleAfterMs, boolean enforceSequence) {
        requireEngine();
        OfExternalFeedPolicy policy = new OfExternalFeedPolicy();
        policy.stale_after_ms = staleAfterMs;
        policy.enforce_sequence = (byte) (enforceSequence ? 1 : 0);
        policy.write();
        check(nativeLib.of_configure_external_feed(engine, policy), "of_configure_external_feed");
    }

    /** Marks external feed reconnecting/degraded state. */
    public void setExternalReconnecting(boolean reconnecting) {
        requireEngine();
        check(
                nativeLib.of_external_set_reconnecting(engine, (byte) (reconnecting ? 1 : 0)),
                "of_external_set_reconnecting");
    }

    /** Re-evaluates external-feed health without ingesting new events. */
    public void externalHealthTick() {
        requireEngine();
        check(nativeLib.of_external_health_tick(engine), "of_external_health_tick");
    }

    /** Convenience overload for ingesting one trade with default metadata. */
    public void ingestTrade(Symbol symbol, long price, long size, int aggressorSide) {
        ingestTrade(symbol, price, size, aggressorSide, 0L, 0L, 0L, DataQualityFlags.NONE);
    }

    /** Ingests one external trade event into runtime processing. */
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

    /** Convenience overload for ingesting one book update with default metadata. */
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

    /** Ingests one external book event into runtime processing. */
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

    /** Returns current book snapshot as JSON string. */
    public String bookSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.BOOK);
    }

    /** Returns current analytics snapshot as JSON string. */
    public String analyticsSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.ANALYTICS);
    }

    /** Returns current signal snapshot as JSON string. */
    public String signalSnapshot(Symbol symbol) {
        return snapshot(symbol, SnapshotKind.SIGNAL);
    }

    /** Returns runtime metrics as JSON string. */
    public String metricsJson() {
        requireEngine();
        PointerByReference out = new PointerByReference();
        IntByReference outLen = new IntByReference(0);
        check(nativeLib.of_get_metrics_json(engine, out, outLen), "of_get_metrics_json");
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

    /** Unsubscribes active subscriptions and destroys native engine handle. */
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
        requireEngine();
        OfSymbol sym = toNativeSymbol(symbol);
        sym.write();

        Memory buffer = new Memory(4096);
        IntByReference length = new IntByReference(4096);

        int rc;
        switch (kind) {
            case BOOK -> rc = nativeLib.of_get_book_snapshot(engine, sym, buffer, length);
            case ANALYTICS -> rc = nativeLib.of_get_analytics_snapshot(engine, sym, buffer, length);
            case SIGNAL -> rc = nativeLib.of_get_signal_snapshot(engine, sym, buffer, length);
            default -> throw new OrderflowException("unknown snapshot kind");
        }

        check(rc, "snapshot");
        int outLen = length.getValue();
        if (outLen <= 0) {
            return "{}";
        }
        return new String(buffer.getByteArray(0, outLen), StandardCharsets.UTF_8);
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

    private enum SnapshotKind {
        BOOK,
        ANALYTICS,
        SIGNAL,
    }
}
