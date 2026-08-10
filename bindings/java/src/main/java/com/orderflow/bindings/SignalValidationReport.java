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
        if (!NativeSignalJson.booleanValue(json, "valid")) {
            throw new OrderflowArgException(NativeSignalJson.nullableString(json, "error"));
        }
        this.moduleId = NativeSignalJson.nullableString(json, "module_id");
        this.evaluatedEvents = NativeSignalJson.longValue(json, "evaluated_events");
        this.labeledEvents = NativeSignalJson.longValue(json, "labeled_events");
        this.missingMarkouts = NativeSignalJson.longValue(json, "missing_markouts");
        this.directionalPredictions = NativeSignalJson.longValue(json, "directional_predictions");
        this.longPredictions = NativeSignalJson.longValue(json, "long_predictions");
        this.shortPredictions = NativeSignalJson.longValue(json, "short_predictions");
        this.neutralPredictions = NativeSignalJson.longValue(json, "neutral_predictions");
        this.blockedPredictions = NativeSignalJson.longValue(json, "blocked_predictions");
        this.correctDirectional = NativeSignalJson.longValue(json, "correct_directional");
        this.incorrectDirectional = NativeSignalJson.longValue(json, "incorrect_directional");
        this.flatMarkouts = NativeSignalJson.longValue(json, "flat_markouts");
        this.averageConfidenceBps = NativeSignalJson.intValue(json, "average_confidence_bps");
        this.directionalAccuracyBps = NativeSignalJson.nullableInt(json, "directional_accuracy_bps");
        this.labelCoverageBps = NativeSignalJson.nullableInt(json, "label_coverage_bps");
        this.warningCount = NativeSignalJson.longValue(json, "warning_count");
        this.rawJson = json;
    }

    /** Parses the stable native report schema into an operational summary. */
    public static SignalValidationReport parse(String json) {
        return new SignalValidationReport(json);
    }
}

final class NativeSignalJson {
    private NativeSignalJson() {}

    static boolean booleanValue(String json, String name) {
        return "true".equals(rawValue(json, name));
    }

    static int intValue(String json, String name) {
        return Math.toIntExact(longValue(json, name));
    }

    static Integer nullableInt(String json, String name) {
        String value = rawValue(json, name);
        return "null".equals(value) ? null : Integer.valueOf(value);
    }

    static long longValue(String json, String name) {
        return Long.parseLong(rawValue(json, name));
    }

    static Long nullableLong(String json, String name) {
        String value = rawValue(json, name);
        return "null".equals(value) ? null : Long.valueOf(value);
    }

    static String nullableString(String json, String name) {
        String value = rawValue(json, name);
        if ("null".equals(value)) {
            return null;
        }
        if (value.length() < 2 || value.charAt(0) != '"') {
            throw new OrderflowException("invalid native signal JSON field: " + name);
        }
        return unescape(value.substring(1, value.length() - 1));
    }

    static String rawValue(String json, String name) {
        if (json == null) {
            throw new OrderflowException("native signal JSON is null");
        }
        String token = "\"" + name + "\"";
        int key = json.indexOf(token);
        if (key < 0) {
            throw new OrderflowException("native signal JSON is missing field: " + name);
        }
        int cursor = json.indexOf(':', key + token.length()) + 1;
        while (cursor > 0 && cursor < json.length() && Character.isWhitespace(json.charAt(cursor))) {
            cursor++;
        }
        if (cursor <= 0 || cursor >= json.length()) {
            throw new OrderflowException("invalid native signal JSON field: " + name);
        }
        if (json.charAt(cursor) == '"') {
            int end = cursor + 1;
            boolean escaped = false;
            while (end < json.length()) {
                char ch = json.charAt(end);
                if (ch == '"' && !escaped) {
                    return json.substring(cursor, end + 1);
                }
                escaped = ch == '\\' && !escaped;
                if (ch != '\\') {
                    escaped = false;
                }
                end++;
            }
            throw new OrderflowException("unterminated native signal JSON string: " + name);
        }
        int end = cursor;
        while (end < json.length() && ",}]".indexOf(json.charAt(end)) < 0) {
            end++;
        }
        return json.substring(cursor, end).trim();
    }

    private static String unescape(String value) {
        StringBuilder out = new StringBuilder(value.length());
        for (int index = 0; index < value.length(); index++) {
            char ch = value.charAt(index);
            if (ch != '\\') {
                out.append(ch);
                continue;
            }
            if (++index >= value.length()) {
                throw new OrderflowException("invalid native signal JSON escape");
            }
            char escaped = value.charAt(index);
            switch (escaped) {
                case '"', '\\', '/' -> out.append(escaped);
                case 'b' -> out.append('\b');
                case 'f' -> out.append('\f');
                case 'n' -> out.append('\n');
                case 'r' -> out.append('\r');
                case 't' -> out.append('\t');
                case 'u' -> {
                    if (index + 4 >= value.length()) {
                        throw new OrderflowException("invalid native signal JSON unicode escape");
                    }
                    out.append((char) Integer.parseInt(value.substring(index + 1, index + 5), 16));
                    index += 4;
                }
                default -> throw new OrderflowException("invalid native signal JSON escape");
            }
        }
        return out.toString();
    }
}
