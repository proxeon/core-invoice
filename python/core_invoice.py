"""Python bindings over the core-invoice C ABI (ctypes). Not a second parser."""

from __future__ import annotations

import ctypes
import os
from ctypes import c_char_p, c_int, c_size_t
from pathlib import Path


def _load():
    root = Path(__file__).resolve().parents[1]
    names = [
        "libcore_invoice_sys.dylib",
        "libcore_invoice_sys.so",
        "core_invoice_sys.dll",
    ]
    search = [
        root / "target" / "debug",
        root / "target" / "release",
    ]
    for d in search:
        for n in names:
            p = d / n
            if p.exists():
                return ctypes.CDLL(str(p))
    raise FileNotFoundError("libcore_invoice_sys not found; cargo build -p core-invoice-sys")


_LIB = None


def _lib():
    global _LIB
    if _LIB is None:
        _LIB = _load()
        _LIB.core_invoice_validate.argtypes = [c_char_p, c_char_p, ctypes.c_void_p, c_size_t]
        _LIB.core_invoice_validate.restype = c_int
    return _LIB


def validate_xml(xml: str, profile: str | None = None) -> int:
    """Return 0 valid, 1 invalid, 2 unreadable / bad args. Same as the CLI."""
    buf = ctypes.create_string_buffer(4096)
    prof = None if profile is None else profile.encode("utf-8")
    return int(
        _lib().core_invoice_validate(xml.encode("utf-8"), prof, buf, 4096)
    )
