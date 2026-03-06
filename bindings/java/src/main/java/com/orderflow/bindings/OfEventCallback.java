package com.orderflow.bindings;

import com.sun.jna.Callback;
import com.sun.jna.Pointer;

/** Low-level JNA callback matching native `of_event_cb`. */
public interface OfEventCallback extends Callback {
    /** Invoked by native runtime when an event is emitted. */
    void invoke(Pointer ev, Pointer userData);
}
