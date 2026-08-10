#!/usr/bin/env python3
"""Unit tests for deterministic low-level binding signature generation."""

from __future__ import annotations

import unittest

from check_api_manifest import DEFAULT_HEADER, DEFAULT_MANIFEST, load_manifest
from generate_binding_signatures import (
    CParameter,
    generated_entries,
    java_type,
    parse_header_declarations,
    python_type,
    render_java,
    render_python,
)


class BindingSignatureGenerationTests(unittest.TestCase):
    """Exercise parsing, pointer depth, arrays, and complete rendering."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.entries = generated_entries(load_manifest(DEFAULT_MANIFEST))
        cls.declarations = parse_header_declarations(DEFAULT_HEADER)

    def test_manifest_symbols_have_complete_header_declarations(self) -> None:
        self.assertEqual(len(self.entries), 94)
        self.assertEqual(
            {entry.name for entry in self.entries},
            set(self.declarations),
        )

    def test_multiline_callback_declaration_is_parsed_exactly(self) -> None:
        subscribe = self.declarations["of_subscribe"]
        self.assertEqual(subscribe.returns, "int32_t")
        self.assertEqual(
            [parameter.c_type for parameter in subscribe.parameters],
            [
                "of_engine_t*",
                "const of_symbol_t*",
                "uint32_t",
                "of_event_cb",
                "void*",
                "of_subscription_t**",
            ],
        )

    def test_python_opaque_output_handle_preserves_pointer_depth(self) -> None:
        self.assertEqual(
            python_type("of_engine_t**"),
            "ctypes.POINTER(ctypes.c_void_p)",
        )
        self.assertEqual(python_type("of_engine_t*"), "ctypes.c_void_p")

    def test_java_context_maps_arrays_and_buffers_explicitly(self) -> None:
        routes = CParameter("const of_execution_route_config_t*", "routes")
        out_buf = CParameter("void*", "out_buf")
        self.assertEqual(
            java_type("of_execution_engine_create_multi", routes, routes.c_type),
            "OfExecutionRouteConfig[]",
        )
        self.assertEqual(
            java_type("of_get_book_snapshot", out_buf, out_buf.c_type),
            "Memory",
        )

    def test_renderers_cover_every_manifest_symbol(self) -> None:
        python = render_python(self.entries, self.declarations)
        java = render_java(self.entries, self.declarations)
        for entry in self.entries:
            self.assertIn(f"lib.{entry.name}.argtypes", python)
            self.assertIn(f" {entry.name}(", java)


if __name__ == "__main__":
    unittest.main()
