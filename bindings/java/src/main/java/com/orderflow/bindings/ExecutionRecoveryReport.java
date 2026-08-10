package com.orderflow.bindings;

/** Bounded, identifier-free summary of read-only OMS recovery. */
public final class ExecutionRecoveryReport {
    /** Native report schema version. */
    public final int schemaVersion;
    /** Checkpoint used for recovery, or null for replay from WAL start. */
    public final Long checkpointId;
    /** Recovered route configuration hash. */
    public final long routeConfigHash;
    /** Whether the recovered kill switch is active. */
    public final boolean killSwitch;
    /** Total recovered order count. */
    public final long orders;
    /** Recovered non-terminal order count. */
    public final long openOrders;
    /** Recovered position snapshot count. */
    public final long positions;
    /** Full-payload commands applied during replay. */
    public final long commandsSeen;
    /** Execution events applied during replay. */
    public final long eventsApplied;
    /** Validated WAL replay range. */
    public final ExecutionRecoveryReplay replay;
    /** Whether venue reconciliation is required before resume. */
    public final boolean venueReconciliationRequired;
    /** Whether submissions are enabled; read-only reports remain disabled. */
    public final boolean submissionsEnabled;
    /** Original native JSON report. */
    public final String rawJson;

    private ExecutionRecoveryReport(String json) {
        this.schemaVersion = NativeSignalJson.intValue(json, "schema_version");
        this.checkpointId = NativeSignalJson.nullableLong(json, "checkpoint_id");
        this.routeConfigHash = NativeSignalJson.longValue(json, "route_config_hash");
        this.killSwitch = NativeSignalJson.booleanValue(json, "kill_switch");
        this.orders = NativeSignalJson.longValue(json, "orders");
        this.openOrders = NativeSignalJson.longValue(json, "open_orders");
        this.positions = NativeSignalJson.longValue(json, "positions");
        this.commandsSeen = NativeSignalJson.longValue(json, "commands_seen");
        this.eventsApplied = NativeSignalJson.longValue(json, "events_applied");
        this.replay = new ExecutionRecoveryReplay(
            NativeSignalJson.longValue(json, "records"),
            NativeSignalJson.longValue(json, "bytes"),
            NativeSignalJson.nullableLong(json, "first_sequence"),
            NativeSignalJson.nullableLong(json, "last_sequence")
        );
        this.venueReconciliationRequired =
            NativeSignalJson.booleanValue(json, "venue_reconciliation_required");
        this.submissionsEnabled = NativeSignalJson.booleanValue(json, "submissions_enabled");
        this.rawJson = json;
    }

    /** Parses the stable native recovery report schema. */
    public static ExecutionRecoveryReport parse(String json) {
        return new ExecutionRecoveryReport(json);
    }
}
