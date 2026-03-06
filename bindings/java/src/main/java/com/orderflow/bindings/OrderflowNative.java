package com.orderflow.bindings;

import com.sun.jna.Library;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.IntByReference;
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

    /** Reads book snapshot JSON into caller buffer. */
    int of_get_book_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads analytics snapshot JSON into caller buffer. */
    int of_get_analytics_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);
    /** Reads signal snapshot JSON into caller buffer. */
    int of_get_signal_snapshot(Pointer engine, OfSymbol symbol, Memory outBuf, IntByReference inoutLen);

    /** Returns metrics JSON pointer and length. */
    int of_get_metrics_json(Pointer engine, PointerByReference outJson, IntByReference outLen);
    /** Frees strings allocated by native library. */
    void of_string_free(Pointer p);
}
