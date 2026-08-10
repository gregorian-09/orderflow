package com.orderflow.bindings;

import java.util.Objects;

/** One typed parameter used to construct a built-in signal module. */
public final class SignalConfigParameter {
    /** Signed integer parameter kind. */ public static final int INTEGER = 1;
    /** Floating-point parameter kind. */ public static final int FLOAT = 2;
    /** Boolean parameter kind. */ public static final int BOOLEAN = 3;
    /** UTF-8 text parameter kind. */ public static final int TEXT = 4;

    /** Descriptor parameter name. */ public final String name;
    /** Native parameter kind. */ public final int kind;
    /** Integer payload. */ public final long integerValue;
    /** Floating-point payload. */ public final double floatValue;
    /** Boolean payload. */ public final boolean booleanValue;
    /** Text payload. */ public final String textValue;

    private SignalConfigParameter(
            String name, int kind, long integerValue, double floatValue,
            boolean booleanValue, String textValue) {
        this.name = requireName(name);
        this.kind = kind;
        this.integerValue = integerValue;
        this.floatValue = floatValue;
        this.booleanValue = booleanValue;
        this.textValue = textValue;
    }

    /** Creates a signed integer parameter. */
    public static SignalConfigParameter integer(String name, long value) {
        return new SignalConfigParameter(name, INTEGER, value, 0.0, false, null);
    }

    /** Creates a finite floating-point parameter. */
    public static SignalConfigParameter floating(String name, double value) {
        if (!Double.isFinite(value)) {
            throw new IllegalArgumentException("signal parameter must be finite");
        }
        return new SignalConfigParameter(name, FLOAT, 0, value, false, null);
    }

    /** Creates a boolean parameter. */
    public static SignalConfigParameter bool(String name, boolean value) {
        return new SignalConfigParameter(name, BOOLEAN, 0, 0.0, value, null);
    }

    /** Creates a UTF-8 text parameter. */
    public static SignalConfigParameter text(String name, String value) {
        return new SignalConfigParameter(
            name, TEXT, 0, 0.0, false, Objects.requireNonNull(value, "value"));
    }

    private static String requireName(String name) {
        Objects.requireNonNull(name, "name");
        if (name.isBlank()) {
            throw new IllegalArgumentException("signal parameter name must not be blank");
        }
        return name;
    }
}
