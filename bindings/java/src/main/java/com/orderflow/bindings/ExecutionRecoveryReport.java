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
        this.schemaVersion = NativeJson.intValue(json, "schema_version");
        this.checkpointId = NativeJson.nullableLong(json, "checkpoint_id");
        this.routeConfigHash = NativeJson.longValue(json, "route_config_hash");
        this.killSwitch = NativeJson.booleanValue(json, "kill_switch");
        this.orders = NativeJson.longValue(json, "orders");
        this.openOrders = NativeJson.longValue(json, "open_orders");
        this.positions = NativeJson.longValue(json, "positions");
        this.commandsSeen = NativeJson.longValue(json, "commands_seen");
        this.eventsApplied = NativeJson.longValue(json, "events_applied");
        this.replay = new ExecutionRecoveryReplay(
            NativeJson.longValue(json, "records"),
            NativeJson.longValue(json, "bytes"),
            NativeJson.nullableLong(json, "first_sequence"),
            NativeJson.nullableLong(json, "last_sequence")
        );
        this.venueReconciliationRequired =
            NativeJson.booleanValue(json, "venue_reconciliation_required");
        this.submissionsEnabled = NativeJson.booleanValue(json, "submissions_enabled");
        this.rawJson = json;
    }

    /** Parses the stable native recovery report schema. */
    public static ExecutionRecoveryReport parse(String json) {
        return new ExecutionRecoveryReport(json);
    }
}
