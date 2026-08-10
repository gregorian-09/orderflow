package com.orderflow.bindings;

import com.sun.jna.Structure;

/** JNA mirror of native {@code of_signal_config_parameter_t}. */
@Structure.FieldOrder({
    "name", "kind", "integer_value", "float_value", "boolean_value", "text_value"
})
public class OfSignalConfigParameter extends Structure {
    /** Parameter name. */ public String name;
    /** Tagged value kind. */ public int kind;
    /** Integer payload. */ public long integer_value;
    /** Floating-point payload. */ public double float_value;
    /** Boolean payload. */ public byte boolean_value;
    /** Text payload. */ public String text_value;
}
