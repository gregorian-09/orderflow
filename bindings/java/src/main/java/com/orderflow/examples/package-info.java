/**
 * Executable examples for the Java binding.
 *
 * <ul>
 *   <li>{@code BasicExample}: subscribe/poll/snapshot baseline flow.</li>
 *   <li>{@code HealthExample}: health stream transitions and quality flags.</li>
 *   <li>{@code ExternalIngestExample}: inject external trade/book updates directly.</li>
 * </ul>
 *
 * <p>Run examples with Maven from repository root:
 * <pre>{@code
 * mvn -q -f bindings/java/pom.xml exec:java -Dexec.mainClass=com.orderflow.examples.BasicExample
 * }</pre>
 */
package com.orderflow.examples;

