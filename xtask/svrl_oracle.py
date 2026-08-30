#!/usr/bin/env python3
"""ConnectingEurope / PINT-MY XSLT SVRL oracle. Env-gated; not the default test.

Saxon is an oracle runner, not a crate dependency. Compare SVRL failed-assert @id
to Fatal Finding.id from `core-invoice validate --format json`. Mapping:
docs/svrl-id-map.md. Ids listed in docs/UNCOVERED.md are expected unmatched when
the artefact fires and we do not.
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SVRL_NS = {"svrl": "http://purl.oclc.org/dsdl/svrl"}
# Artefact ids are compared case-insensitively; PINT-MY zip uses lowercase.
ID_TOKEN = re.compile(
    r"\b((?:BR|PEPPOL-EN16931|PEPPOL-COMMON|ALIGNED-IBRP|IBR)[A-Z0-9\-]+)\b",
    re.I,
)


def canonical(s: str) -> str:
    """Case-fold. ALIGNED-IBRP-SA-08-MY and aligned-ibrp-sa-08 compare equal."""
    t = s.strip().upper()
    if t.endswith("-MY") and t.startswith("ALIGNED-IBRP"):
        t = t[: -len("-MY")]
    return t


def require_spec() -> bool:
    return os.environ.get("CORE_INVOICE_REQUIRE_SPEC") == "1"


def env_on() -> bool:
    return require_spec() or bool(os.environ.get("SVRL_ORACLE"))


def load_map(path: Path) -> dict[str, str]:
    """artefact @id -> our Finding.id (canonical keys)."""
    mapping: dict[str, str] = {}
    if not path.is_file():
        return mapping
    for line in path.read_text().splitlines():
        if not line.startswith("|") or "Artefact" in line or line.startswith("|---"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 2:
            continue
        art, ours = cells[0], cells[1]
        if not art or art == "—" or "UNCOVERED" in art or art.startswith("ids in"):
            continue
        if ours in {"", "—", "-"}:
            continue
        mapping[canonical(art)] = canonical(ours)
    return mapping


def load_uncovered(path: Path) -> set[str]:
    ids: set[str] = set()
    if not path.is_file():
        return ids
    text = path.read_text()
    for m in ID_TOKEN.finditer(text):
        ids.add(canonical(m.group(1)))
    # Range PEPPOL-COMMON-R041–R053 (en-dash or hyphen).
    for a, b in re.findall(
        r"PEPPOL-COMMON-R0(\d{2})[–-]R0(\d{2})", text, flags=re.I
    ):
        for n in range(int(a), int(b) + 1):
            ids.add(canonical(f"PEPPOL-COMMON-R0{n:02d}"))
    return ids


def parse_svrl_failed_ids(xml: str) -> set[str]:
    import xml.etree.ElementTree as ET

    root = ET.fromstring(xml)
    ids: set[str] = set()
    for el in root.findall(".//{http://purl.oclc.org/dsdl/svrl}failed-assert"):
        raw = el.get("id") or ""
        flag = (el.get("flag") or "").lower()
        if flag in {"warning", "information"}:
            continue
        if raw:
            ids.add(canonical(raw))
    return ids


def our_fatal_ids(cli: Path, xml_path: Path, profile: str | None) -> set[str]:
    cmd = [str(cli), "validate", "--format", "json"]
    if profile:
        cmd.extend(["--profile", profile])
    else:
        cmd.extend(["--profile", "auto"])
    cmd.append(str(xml_path))
    proc = subprocess.run(cmd, capture_output=True, text=True)
    # Exit 0/1 still prints JSON on stdout.
    raw = proc.stdout.strip()
    if not raw.startswith("{"):
        raise RuntimeError(f"validate json missing for {xml_path}: {raw!r} {proc.stderr}")
    data = json.loads(raw)
    ids: set[str] = set()
    for f in data.get("findings", []):
        if str(f.get("severity", "")).lower() != "fatal":
            continue
        ids.add(canonical(str(f.get("id", ""))))
    return ids


def diff_sets(
    svrl: set[str],
    ours: set[str],
    mapping: dict[str, str],
    uncovered: set[str],
) -> tuple[set[str], set[str]]:
    """Return (unexpected extra on us, unexpected missing vs SVRL)."""
    mapped_ours = {mapping.get(i, i) for i in svrl}
    mapped_svrl_as_ours = mapped_ours
    extra = ours - mapped_svrl_as_ours - uncovered
    # Our extras that 1:1-map back to an artefact id we simply did not see: still extra.
    missing = set()
    for art in svrl:
        want = mapping.get(art, art)
        if want in ours:
            continue
        if art in uncovered or want in uncovered:
            continue
        missing.add(art)
    return extra, missing


def saxon_cmd(saxon: str | None, jar: str | None, xslt: Path, xml: Path, out: Path) -> list[str]:
    if saxon:
        # Saxon-HE CLI: Transform -s:in -xsl:sheet -o:out
        return [saxon, f"-s:{xml}", f"-xsl:{xslt}", f"-o:{out}"]
    assert jar
    return [
        "java",
        "-jar",
        jar,
        f"-s:{xml}",
        f"-xsl:{xslt}",
        f"-o:{out}",
    ]


def find_cli() -> Path | None:
    release = ROOT / "target/release/core-invoice"
    debug = ROOT / "target/debug/core-invoice"
    if debug.is_file():
        return debug
    if release.is_file():
        return release
    return shutil.which("core-invoice") and Path(shutil.which("core-invoice"))  # type: ignore[arg-type]


def default_en_files() -> list[Path]:
    base = ROOT / "refers/en16931/ubl/examples"
    names = [
        "ubl-tc434-example1.xml",
        "ubl-tc434-example5.xml",
        "sample-discount-price.xml",
    ]
    out = [base / n for n in names if (base / n).is_file()]
    mutant = ROOT / "xtask/testdata/missing-issue-date.xml"
    if mutant.is_file():
        out.append(mutant)
    return out


def self_test() -> int:
    mapping = {"BR-CO-17": "BR-CO-17", "PEPPOL-EN16931-R010": "PEPPOL-EN16931-R010"}
    uncovered = {"BR-17"}
    extra, missing = diff_sets(
        {"BR-CO-17", "BR-17"},
        {"BR-CO-17"},
        mapping,
        uncovered,
    )
    assert extra == set(), extra
    assert missing == set(), missing
    extra, missing = diff_sets({"BR-03"}, {"BR-03", "CORE-SPEC-01"}, mapping, uncovered)
    assert "CORE-SPEC-01" in extra
    print("self-test ok")
    return 0


def main(argv: list[str]) -> int:
    if argv[1:] == ["--self-test"]:
        return self_test()
    if not env_on():
        print("skip: set CORE_INVOICE_REQUIRE_SPEC=1 or SVRL_ORACLE=1")
        return 0

    xslt_en = ROOT / "refers/en16931/ubl/xslt/EN16931-UBL-validation.xslt"
    xslt_dir = ROOT / "refers/en16931/ubl/xslt"
    if not xslt_dir.exists():
        print(f"missing {xslt_dir}; run task spec", file=sys.stderr)
        return 1 if require_spec() else 0

    saxon = shutil.which("saxon") or shutil.which("saxonb-xslt") or shutil.which("transform")
    jar = os.environ.get("SAXON_JAR")
    if not saxon and not jar:
        print("skip: no saxon CLI / SAXON_JAR")
        return 1 if require_spec() else 0

    if os.environ.get("MUSTANG_JAR"):
        print("MUSTANG_JAR set — VAT CII only; do not send SST through Mustang")

    mapping = load_map(ROOT / "docs/svrl-id-map.md")
    uncovered = load_uncovered(ROOT / "docs/UNCOVERED.md")
    cli = find_cli()
    if cli is None:
        print("building core-invoice-cli for oracle compare", file=sys.stderr)
        r = subprocess.run(
            ["cargo", "build", "-p", "core-invoice-cli", "--offline"],
            cwd=ROOT,
        )
        if r.returncode != 0:
            return r.returncode
        cli = ROOT / "target/debug/core-invoice"

    files = [Path(a) for a in argv[1:] if not a.startswith("-")]
    if not files:
        files = default_en_files()
    if not files:
        print("no XML files to compare (pass paths or fetch refers/)", file=sys.stderr)
        return 1 if require_spec() else 0

    if not xslt_en.is_file():
        print(f"missing {xslt_en}", file=sys.stderr)
        return 1 if require_spec() else 0

    failed = 0
    for xml in files:
        if not xml.is_file():
            print(f"missing {xml}", file=sys.stderr)
            failed += 1
            continue
        with tempfile.NamedTemporaryFile(suffix=".svrl.xml", delete=False) as tmp:
            out = Path(tmp.name)
        try:
            cmd = saxon_cmd(saxon, jar, xslt_en, xml, out)
            proc = subprocess.run(cmd, capture_output=True, text=True)
            if proc.returncode != 0:
                print(f"saxon failed on {xml}: {proc.stderr}", file=sys.stderr)
                failed += 1
                continue
            svrl = parse_svrl_failed_ids(out.read_text(errors="replace"))
            ours = our_fatal_ids(cli, xml, None)
            extra, missing = diff_sets(svrl, ours, mapping, uncovered)
            print(f"{xml.name}: svrl={sorted(svrl)} ours={sorted(ours)}")
            if extra or missing:
                print(f"  unexpected extra={sorted(extra)} missing={sorted(missing)}", file=sys.stderr)
                failed += 1
            else:
                print("  comparable")
        finally:
            out.unlink(missing_ok=True)

    # Peppol BIS v3.0.20 tree has Schematron (.sch), not compiled XSLT.
    peppol_sch = ROOT / "refers/peppol-bis-invoice-3/rules/sch/PEPPOL-EN16931-UBL.sch"
    if peppol_sch.is_file():
        print(f"note: Peppol BIS path is {peppol_sch} (Schematron; no compiled XSLT in this pin)")

    my_xslt = ROOT / (
        "refers/pint-my-1.3.0/unpacked/trn-invoice/schematron/"
        "PINT-UBL-validation-preprocessed.xslt"
    )
    sa = ROOT / "refers/pint-my-1.3.0/unpacked/trn-invoice/example/Invoice-Sample-SA_1.3.0.xml"
    if my_xslt.is_file() and sa.is_file() and ("--pint-my" in argv or not argv[1:]):
        with tempfile.NamedTemporaryFile(suffix=".svrl.xml", delete=False) as tmp:
            out = Path(tmp.name)
        try:
            cmd = saxon_cmd(saxon, jar, my_xslt, sa, out)
            proc = subprocess.run(cmd, capture_output=True, text=True)
            if proc.returncode != 0:
                print(f"saxon PINT-MY failed: {proc.stderr}", file=sys.stderr)
                failed += 1
            else:
                svrl = parse_svrl_failed_ids(out.read_text(errors="replace"))
                ours = our_fatal_ids(cli, sa, "pint-my")
                extra, missing = diff_sets(svrl, ours, mapping, uncovered)
                print(f"PINT-MY SA: svrl={sorted(svrl)} ours={sorted(ours)}")
                if extra or missing:
                    print(f"  unexpected extra={sorted(extra)} missing={sorted(missing)}", file=sys.stderr)
                    failed += 1
                else:
                    print("  comparable")
        finally:
            out.unlink(missing_ok=True)

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
