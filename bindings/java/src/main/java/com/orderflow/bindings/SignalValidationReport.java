package com.orderflow.bindings;

/** Parsed operational summary of a native signal replay-validation report. */
public final class SignalValidationReport {
    /** Native signal module identifier. */ public final String moduleId;
    /** Number of replay observations evaluated. */ public final long evaluatedEvents;
    /** Number of observations with future markout labels. */ public final long labeledEvents;
    /** Number of observations without a future markout. */ public final long missingMarkouts;
    /** Number of directional predictions scored. */ public final long directionalPredictions;
    /** Number of long predictions. */ public final long longPredictions;
    /** Number of short predictions. */ public final long shortPredictions;
    /** Number of neutral predictions. */ public final long neutralPredictions;
    /** Number of quality-blocked predictions. */ public final long blockedPredictions;
    /** Number of correct directional predictions. */ public final long correctDirectional;
    /** Number of incorrect directional predictions. */ public final long incorrectDirectional;
    /** Number of flat future markouts. */ public final long flatMarkouts;
    /** Average emitted confidence in basis points. */ public final int averageConfidenceBps;
    /** Directional accuracy in basis points, or null when unscored. */ public final Integer directionalAccuracyBps;
    /** Labeled-event coverage in basis points, or null for empty input. */ public final Integer labelCoverageBps;
    /** Number of structured warnings in the full report. */ public final long warningCount;
    /** Original JSON including config, retained samples, and structured warnings. */ public final String rawJson;

    private SignalValidationReport(String json) {
        if (!NativeJson.booleanValue(json, "valid")) {
            throw new OrderflowArgException(NativeJson.nullableString(json, "error"));
        }
        this.moduleId = NativeJson.nullableString(json, "module_id");
        this.evaluatedEvents = NativeJson.longValue(json, "evaluated_events");
        this.labeledEvents = NativeJson.longValue(json, "labeled_events");
        this.missingMarkouts = NativeJson.longValue(json, "missing_markouts");
        this.directionalPredictions = NativeJson.longValue(json, "directional_predictions");
        this.longPredictions = NativeJson.longValue(json, "long_predictions");
        this.shortPredictions = NativeJson.longValue(json, "short_predictions");
        this.neutralPredictions = NativeJson.longValue(json, "neutral_predictions");
        this.blockedPredictions = NativeJson.longValue(json, "blocked_predictions");
        this.correctDirectional = NativeJson.longValue(json, "correct_directional");
        this.incorrectDirectional = NativeJson.longValue(json, "incorrect_directional");
        this.flatMarkouts = NativeJson.longValue(json, "flat_markouts");
        this.averageConfidenceBps = NativeJson.intValue(json, "average_confidence_bps");
        this.directionalAccuracyBps = NativeJson.nullableInt(json, "directional_accuracy_bps");
        this.labelCoverageBps = NativeJson.nullableInt(json, "label_coverage_bps");
        this.warningCount = NativeJson.longValue(json, "warning_count");
        this.rawJson = json;
    }

    /** Parses the stable native report schema into an operational summary. */
    public static SignalValidationReport parse(String json) {
        return new SignalValidationReport(json);
    }
}
