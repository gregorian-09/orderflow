package com.orderflow.bindings;

import java.util.List;
import java.util.Objects;

/** Registry identifier and parameters for a built-in signal module. */
public final class SignalConfig {
    /** Stable signal descriptor identifier. */ public final String signalId;
    /** Ordered signal parameters. */ public final List<SignalConfigParameter> parameters;

    /** Creates registry configuration with explicit parameters. */
    public SignalConfig(String signalId, List<SignalConfigParameter> parameters) {
        Objects.requireNonNull(signalId, "signalId");
        if (signalId.isBlank()) {
            throw new IllegalArgumentException("signalId must not be blank");
        }
        this.signalId = signalId;
        this.parameters = List.copyOf(Objects.requireNonNull(parameters, "parameters"));
    }

    /** Creates registry configuration using descriptor defaults. */
    public static SignalConfig defaults(String signalId) {
        return new SignalConfig(signalId, List.of());
    }
}
