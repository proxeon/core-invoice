"""ctypes surface is validate_xml only. Skip if libcore_invoice_sys is not built."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    from core_invoice import validate_xml
except FileNotFoundError as e:
    print(f"skip: {e}")
    raise SystemExit(0) from e

XML = """<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID></cbc:ID></Invoice>"""


def main() -> None:
    code, _msg = validate_xml(XML, "en16931")
    assert code in (0, 1, 2), code
    assert code != 0, "empty ID must not be valid"
    bad, _ = validate_xml("not xml", None)
    assert bad == 2, bad
    print("ok")


if __name__ == "__main__":
    main()
