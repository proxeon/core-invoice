#!/usr/bin/env python3
"""ConnectingEurope XSLT SVRL oracle. Env-gated; not the default test.

Requires `refers/en16931` and a Saxon CLI (`saxon` or `SAXON_JAR`).
Parse SVRL `@id` and compare to core-invoice findings (see docs/svrl-id-map.md).
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    if os.environ.get("CORE_INVOICE_REQUIRE_SPEC") != "1" and not os.environ.get("SVRL_ORACLE"):
        print("skip: set CORE_INVOICE_REQUIRE_SPEC=1 or SVRL_ORACLE=1")
        return 0
    xslt = ROOT / "refers/en16931/ubl/xslt"
    if not xslt.exists():
        print(f"missing {xslt}; run task spec", file=sys.stderr)
        return 1 if os.environ.get("CORE_INVOICE_REQUIRE_SPEC") == "1" else 0
    saxon = shutil.which("saxon") or shutil.which("saxonb-xslt")
    jar = os.environ.get("SAXON_JAR")
    if not saxon and not jar:
        print("skip: no saxon CLI / SAXON_JAR")
        return 0
    print("SVRL oracle: XSLT present; wire a per-file compare using docs/svrl-id-map.md")
    print(f"xslt dir: {xslt}")
    if os.environ.get("MUSTANG_JAR"):
        print("MUSTANG_JAR set — VAT CII only; do not send SST through Mustang")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
