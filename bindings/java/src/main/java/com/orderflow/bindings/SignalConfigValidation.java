package com.orderflow.bindings;

/** Parsed result of native signal registry configuration validation. */
public final class SignalConfigValidation {
    /** Signal identifier that was validated. */ public final String signalId;
    /** Whether registry validation accepted the configuration. */ public final boolean valid;
    /** Validation diagnostic, or null on success. */ public final String error;
    /** Original native JSON document. */ public final String rawJson;

    SignalConfigValidation(String signalId, boolean valid, String error, String rawJson) {
        this.signalId = signalId;
        this.valid = valid;
        this.error = error;
        this.rawJson = rawJson;
    }
}
