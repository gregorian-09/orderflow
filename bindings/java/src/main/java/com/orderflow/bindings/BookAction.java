package com.orderflow.bindings;

/** Book action constants used by external ingest APIs. */
public final class BookAction {
    private BookAction() {}

    /** Insert or update a book level. */
    public static final int UPSERT = 0;
    /** Delete a book level. */
    public static final int DELETE = 1;
}
