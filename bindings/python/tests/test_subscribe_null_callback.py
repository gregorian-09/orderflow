"""Regression tests for callback handling in subscribe()."""

import ctypes
import unittest

from orderflow import Engine, Symbol


class _StubLib:
    def __init__(self) -> None:
        self.seen_callback = None

    def of_subscribe(self, _engine, _symbol, _stream_kind, callback, _user, _out_sub):
        self.seen_callback = callback
        return 0


class _StubFFI:
    def __init__(self) -> None:
        self.lib = _StubLib()


class SubscribeNullCallbackTest(unittest.TestCase):
    def test_subscribe_without_callback_uses_typed_null_pointer(self) -> None:
        engine = Engine.__new__(Engine)
        engine._ffi = _StubFFI()
        engine._engine = ctypes.c_void_p(1)
        engine._subs = []
        engine._callbacks = []
        engine._alive = True
        engine._cfg_cstr = {}

        engine.subscribe(Symbol("CME", "ESM6", 10))

        seen = engine._ffi.lib.seen_callback
        self.assertIsNotNone(seen)
        self.assertFalse(bool(seen))
        self.assertEqual(len(engine._callbacks), 0)


if __name__ == "__main__":
    unittest.main()
