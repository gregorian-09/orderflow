/**
 * Java binding for the Orderflow native runtime.
 *
 * <p>This package exposes a JNA-based wrapper over the stable C ABI exported by {@code of_ffi_c}.
 * It is designed for low-latency ingestion and snapshot access in JVM applications.
 *
 * <h2>Primary Entry Point</h2>
 * <ul>
 *   <li>{@link com.orderflow.bindings.OrderflowEngine} for lifecycle, subscription, polling,
 *       external ingest, and snapshot APIs.</li>
 * </ul>
 *
 * <h2>Data Model</h2>
 * <ul>
 *   <li>{@link com.orderflow.bindings.Symbol} identifies venue + instrument + depth intent.</li>
 *   <li>{@link com.orderflow.bindings.EngineConfig} controls runtime behavior.</li>
 *   <li>Constants classes ({@link com.orderflow.bindings.StreamKind},
 *       {@link com.orderflow.bindings.Side}, {@link com.orderflow.bindings.BookAction},
 *       {@link com.orderflow.bindings.DataQualityFlags}) provide stable numeric mappings.</li>
 * </ul>
 *
 * <h2>Native Library Resolution</h2>
 * <ol>
 *   <li>Explicit constructor path in {@link com.orderflow.bindings.OrderflowEngine}.</li>
 *   <li>{@code ORDERFLOW_LIBRARY_PATH} environment variable.</li>
 *   <li>Default local debug target path.</li>
 * </ol>
 *
 * <h2>Threading/Callbacks</h2>
 * <p>Event callbacks are delivered during {@code pollOnce(...)} and during external ingest calls.
 * Keep callback handlers fast and non-blocking.
 *
 * <h2>Project Docs</h2>
 * <ul>
 *   <li><a href="https://github.com/gregorian-09/orderflow/tree/main/docs/handbook">Handbook</a></li>
 *   <li><a href="https://github.com/gregorian-09/orderflow/tree/main/docs/api">API Reference</a></li>
 *   <li><a href="https://github.com/gregorian-09/orderflow/tree/main/docs/bindings/java.md">Java Binding Guide</a></li>
 * </ul>
 */
package com.orderflow.bindings;
