package com.orderflow.bindings;

/** Minimal parser for bounded, flat native JSON report fields. */
final class NativeJson {
    private NativeJson() {}

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
            throw new OrderflowException("invalid native JSON field: " + name);
        }
        return unescape(value.substring(1, value.length() - 1));
    }

    static String rawValue(String json, String name) {
        if (json == null) {
            throw new OrderflowException("native JSON is null");
        }
        String token = "\"" + name + "\"";
        int key = json.indexOf(token);
        if (key < 0) {
            throw new OrderflowException("native JSON is missing field: " + name);
        }
        int cursor = json.indexOf(':', key + token.length()) + 1;
        while (cursor > 0 && cursor < json.length() && Character.isWhitespace(json.charAt(cursor))) {
            cursor++;
        }
        if (cursor <= 0 || cursor >= json.length()) {
            throw new OrderflowException("invalid native JSON field: " + name);
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
            throw new OrderflowException("unterminated native JSON string: " + name);
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
                throw new OrderflowException("invalid native JSON escape");
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
                        throw new OrderflowException("invalid native JSON unicode escape");
                    }
                    out.append((char) Integer.parseInt(value.substring(index + 1, index + 5), 16));
                    index += 4;
                }
                default -> throw new OrderflowException("invalid native JSON escape");
            }
        }
        return out.toString();
    }
}
