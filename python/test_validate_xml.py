"""ctypes surface is validate_xml only. Fail if libcore_invoice_sys is not built.

Prerequisite: ``cargo build -p core-invoice-sys --locked``.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from core_invoice import validate_xml  # noqa: E402  FileNotFoundError if dylib missing

EMPTY_ID = """<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID></cbc:ID></Invoice>"""
DTD = "<!DOCTYPE foo [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><Invoice/>"
PEPPOL = ROOT / "testdata/peppol-bis-invoice-3/rules/examples/base-example.xml"


def main() -> None:
    code, msg = validate_xml(EMPTY_ID, "en16931")
    assert code == 1, code
    assert msg.strip(), "invalid must fill the C error buffer"

    bad, bad_msg = validate_xml("not xml", None)
    assert bad == 2, bad
    assert bad_msg.strip(), "unreadable must fill the C error buffer"

    unk, unk_msg = validate_xml(EMPTY_ID, "xrechnung")
    assert unk == 2, unk
    assert "unknown profile" in unk_msg, unk_msg
    assert "en16931" in unk_msg, unk_msg

    dtd, dtd_msg = validate_xml(DTD, None)
    assert dtd == 2, dtd
    assert "DTD" in dtd_msg, dtd_msg

    if not PEPPOL.is_file():
        raise SystemExit(f"missing {PEPPOL}")
    xml = PEPPOL.read_text(encoding="utf-8")
    ok, ok_msg = validate_xml(xml, "peppol")
    assert ok == 0, (ok, ok_msg)
    assert ok_msg == "", ok_msg

    print("ok")


if __name__ == "__main__":
    main()
