package com.orderflow.bindings;

import com.sun.jna.Library;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.LongByReference;
import com.sun.jna.ptr.PointerByReference;

/** JNA mapping for the exported Orderflow C ABI. */
public interface OrderflowNative extends Library {
    /** Loads the native library from a concrete path. */
    static OrderflowNative load(String path) {
        return Native.load(path, OrderflowNative.class);
    }

    /** Returns ABI version. */
    int of_api_version();
    /** Returns static build info string. */
    String of_build_info();
    /** Returns execution ABI version. */
    int of_execution_api_version();
    /** Inspects execution WAL file integrity. */
    int of_execution_wal_integrity_report(String path, OfExecutionWalIntegrityReport outReport);
    /** Inspects segmented execution WAL directory integrity. */
    int of_execution_segmented_wal_integrity_report(String root, OfExecutionSegmentedWalIntegrityReport outReport);
    /** Inspects execution checkpoint store directory integrity. */
    int of_execution_checkpoint_store_integrity_report(String root, OfExecutionCheckpointStoreIntegrityReport outReport);

    /** Creates execution engine instance. */
    int of_execution_engine_create(OfExecutionRouteConfig cfg, PointerByReference outEngine);
    /** Creates execution engine instance with multiple route/account/symbol configs. */
    int of_execution_engine_create_multi(OfExecutionRouteConfig[] routes, int routeCount, PointerByReference outEngine);
    /** Starts execution engine. */
    int of_execution_engine_start(Pointer engine);
    /** Stops execution engine. */
    int of_execution_engine_stop(Pointer engine);
    /** Destroys execution engine. */
    void of_execution_engine_destroy(Pointer engine);
    /** Submits execution order. */
    int of_execution_submit_order(Pointer engine, OfExecutionOrderRequest req, OfExecutionEvent[] outEvents, IntByReference inoutLen);
    /** Cancels execution order. */
    int of_execution_cancel_order(Pointer engine, OfExecutionCancelRequest req, OfExecutionEvent[] outEvents, IntByReference inoutLen);
    /** Amends execution order. */
    int of_execution_amend_order(Pointer engine, OfExecutionAmendRequest req, OfExecutionEvent[] outEvents, IntByReference inoutLen);
    /** Polls execution events. */
    int of_execution_poll(Pointer engine, OfExecutionEvent[] outEvents, IntByReference inoutLen);
    /** Gets execution order state. */
    int of_execution_get_order_state(Pointer engine, String clientOrderId, OfExecutionOrderState outState);
    /** Gets execution health. */
    int of_execution_health(Pointer engine, OfExecutionHealth outHealth);
    /** Gets execution metrics. */
    int of_execution_metrics(Pointer engine, OfExecutionMetrics outMetrics);
    /** Creates concurrent execution worker. */
    int of_execution_concurrent_engine_create_multi(OfExecutionRouteConfig[] routes, int routeCount, OfExecutionConcurrentConfig config, PointerByReference outEngine);
    /** Destroys concurrent execution worker. */
    void of_execution_concurrent_engine_destroy(Pointer engine);
    /** Requests concurrent execution worker stop. */
    int of_execution_concurrent_stop(Pointer engine, LongByReference outSequence);
    /** Queues concurrent submit command. */
    int of_execution_concurrent_submit_order(Pointer engine, OfExecutionOrderRequest req, LongByReference outSequence);
    /** Queues concurrent cancel command. */
    int of_execution_concurrent_cancel_order(Pointer engine, OfExecutionCancelRequest req, LongByReference outSequence);
    /** Queues concurrent amend command. */
    int of_execution_concurrent_amend_order(Pointer engine, OfExecutionAmendRequest req, LongByReference outSequence);
    /** Queues concurrent poll command. */
    int of_execution_concurrent_poll(Pointer engine, LongByReference outSequence);
    /** Attempts to receive one concurrent command report. */
    int of_execution_concurrent_try_recv_report(Pointer engine, OfExecutionCommandReport outReport, OfExecutionEvent[] outEvents, IntByReference inoutLen);

    /** Creates engine instance. */
    int of_engine_create(OfEngineConfig cfg, PointerByReference outEngine);
    /** Starts engine. */
    int of_engine_start(Pointer engine);
    /** Stops engine. */
    int of_engine_stop(Pointer engine);
    /** Destroys engine handle. */
    void of_engine_destroy(Pointer engine);

    /** Subscribes symbol stream with optional callback. */
    int of_subscribe(Pointer engine, OfSymbol symbol, int kind, OfEventCallback cb, Pointer userData, PointerByReference outSub);
    /** Unsubscribes by token. */
    int of_unsubscribe(Pointer sub);
    /** Unsubscribes by symbol. */
    int of_unsubscribe_symbol(Pointer engine, OfSymbol symbol);
    /** Resets symbol session state. */
    int of_reset_symbol_session(Pointer engine, OfSymbol symbol);
    /** Injects trade event. */
    int of_ingest_trade(Pointer engine, OfTrade trade, int qualityFlags);
    /** Injects book event. */
    int of_ingest_book(Pointer engine, OfBook book, int qualityFlags);
    /** Configures external feed policy. */
    int of_configure_external_feed(Pointer engine, OfExternalFeedPolicy policy);
    /** Sets reconnecting state for external feed. */
    int of_external_set_reconnecting(Pointer engine, byte reconnecting);
    /** Triggers external-feed health tick. */
    int of_external_health_tick(Pointer engine);
    /** Polls adapter once. */
    int of_engine_poll_once(Pointer engine, int qualityFlags);
    /** Sets tickbar aggregation interval for new per-symbol accumulators. */
    int of_engine_set_tickbar_interval(Pointer engine, long intervalNs);

    /** Reads book snapshot JSON into caller buffer. */
    int of_get_book_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads book analytics snapshot JSON into caller buffer. */
    int of_get_book_analytics_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Computes weighted average price for order of qty walking the book. */
    int of_compute_weighted_average_price(Pointer engine, OfSymbol symbol, long qty, Memory outBuf, IntByReference inoutLen);
    /** Computes depth slope over first `levels` price levels. */
    int of_compute_depth_slope(Pointer engine, OfSymbol symbol, int levels, Memory outBuf, IntByReference inoutLen);
    /** Reads mid price JSON. */
    int of_get_mid_price(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads effective spread in bps. */
    int of_get_effective_spread_bps(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads half-spread cost in bps over `window` trades. */
    int of_get_half_spread_cost_bps(Pointer engine, OfSymbol symbol, int window, Memory outBuf, IntByReference inoutLen);
    /** Reads realised spread in bps for trade `holdTicks` ago. */
    int of_get_realised_spread_bps(Pointer engine, OfSymbol symbol, int holdTicks, Memory outBuf, IntByReference inoutLen);
    /** Reads book-event analytics snapshot JSON. */
    int of_get_book_event_analytics(Pointer engine, OfSymbol symbol, long windowNs, Memory outBuf, IntByReference inoutLen);
    /** Reads resiliency snapshot JSON. */
    int of_get_resiliency_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_vpin_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_kyle_lambda_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_amihud_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_cvd_enhancement_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_pattern_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_volatility_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_noise_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_hasbrouck_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_almgren_chriss_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_spread_decomp_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_acd_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_regime_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_kinetic_energy_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_dark_pool_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_options_flow_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_futures_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_vol_signature_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_agent_type_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_dark_lit_correlation_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_institutional_flow_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    int of_get_oi_analysis_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads analytics snapshot JSON into caller buffer. */
    int of_get_analytics_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads derived analytics snapshot JSON into caller buffer. */
    int of_get_derived_analytics_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads session candle snapshot JSON into caller buffer. */
    int of_get_session_candle_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads rolling interval candle snapshot JSON into caller buffer. */
    int of_get_interval_candle_snapshot(Pointer engine, OfSymbol symbol, long windowNs, Memory outBuf, IntByReference inoutLen);
    /** Reads signal snapshot JSON into caller buffer. */
    int of_get_signal_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads completed bar series JSON array into caller buffer. */
    int of_get_bar_series(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);

    int of_compute_lob_features(Pointer engine, OfSymbol symbol, double tradeImbalance, double cancelRate, double arrivalRate, Memory outBuf, IntByReference inoutLen);

    int of_engine_set_analytics_config(Pointer engine, Pointer config);

    /** Returns metrics JSON pointer and length. */
    int of_get_metrics_json(Pointer engine, PointerByReference outJson, IntByReference outLen);
    /** Frees strings allocated by native library. */
    void of_string_free(Pointer p);
}
