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
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SAXON_JAR_LOCAL = ROOT / "xtask/.saxon/Saxon-HE.jar"
SAXON_JAR_URL = (
    "https://repo1.maven.org/maven2/net/sf/saxon/Saxon-HE/10.9/Saxon-HE-10.9.jar"
)
SVRL_NS = {"svrl": "http://purl.oclc.org/dsdl/svrl"}
# Artefact ids are compared case-insensitively; PINT-MY zip uses lowercase.
ID_TOKEN = re.compile(
    r"\b((?:BR|PEPPOL-EN16931|PEPPOL-COMMON|ALIGNED-IBRP|IBR|UBL-CR|UBL-SR|UBL-DT)[A-Z0-9\-]+)\b",
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


def load_uncovered_text(text: str) -> set[str]:
    """Ids in the fenced block under `## Oracle expected-unmatched` only. Prose is not scanned."""
    ids: set[str] = set()
    m = re.search(
        r"^## Oracle expected-unmatched\s*\n+```[^\n]*\n(.*?)```",
        text,
        re.M | re.S,
    )
    if not m:
        return ids
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        tok = line.split()[0].strip(",")
        for hit in ID_TOKEN.finditer(tok):
            ids.add(canonical(hit.group(1)))
    return ids


def load_uncovered(path: Path) -> set[str]:
    if not path.is_file():
        return set()
    return load_uncovered_text(path.read_text())


def syntax_ids(ids: set[str]) -> set[str]:
    """UBL-CR/SR/DT are formats/unmapped, not the semantic compare."""
    return {
        i
        for i in ids
        if i.startswith("UBL-CR") or i.startswith("UBL-SR") or i.startswith("UBL-DT")
    }


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


def our_fatal_ids(
    cli: Path, xml_path: Path, profile: str | None
) -> tuple[set[str], str]:
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
    return ids, str(data.get("profile") or "")


def extras_artefact_cannot_emit(extra: set[str], artefact: str, profile: str) -> set[str]:
    """EN XSLT cannot emit Peppol/PINT extras; drop those from unexpected extra."""
    if artefact == "en" and profile in {"peppol", "pint", "pint-my"}:
        return {
            i
            for i in extra
            if i.startswith("PEPPOL-") or i.startswith("ALIGNED-") or i.startswith("IBR-")
        }
    return set()


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


def java_works(java: str) -> bool:
    try:
        r = subprocess.run([java, "-version"], capture_output=True, text=True)
    except OSError:
        return False
    text = (r.stdout or "") + (r.stderr or "")
    return r.returncode == 0 and "Unable to locate a Java Runtime" not in text


def ensure_saxon_jar() -> Path:
    if SAXON_JAR_LOCAL.is_file() and SAXON_JAR_LOCAL.stat().st_size > 1_000_000:
        return SAXON_JAR_LOCAL
    SAXON_JAR_LOCAL.parent.mkdir(parents=True, exist_ok=True)
    cached = Path("/tmp/Saxon-HE.jar")
    if cached.is_file() and cached.stat().st_size > 1_000_000:
        shutil.copy2(cached, SAXON_JAR_LOCAL)
        return SAXON_JAR_LOCAL
    print(f"fetching Saxon-HE → {SAXON_JAR_LOCAL}", file=sys.stderr)
    subprocess.run(
        ["curl", "-fsSL", "-o", str(SAXON_JAR_LOCAL), SAXON_JAR_URL],
        check=True,
    )
    return SAXON_JAR_LOCAL


def to_container(path: Path) -> str:
    rel = path.resolve().relative_to(ROOT)
    return "/work/" + rel.as_posix()


def saxon_cmd(xslt: Path, xml: Path, out: Path) -> list[str]:
    """Prefer local saxon/java; otherwise Docker Compose eclipse-temurin + Saxon-HE."""
    cli = shutil.which("saxon") or shutil.which("saxonb-xslt") or shutil.which("transform")
    if cli:
        return [cli, f"-s:{xml}", f"-xsl:{xslt}", f"-o:{out}"]
    jar = os.environ.get("SAXON_JAR")
    java = shutil.which("java")
    if jar and java and java_works(java):
        return [java, "-jar", jar, f"-s:{xml}", f"-xsl:{xslt}", f"-o:{out}"]
    ensure_saxon_jar()
    if java and java_works(java):
        return [java, "-jar", str(SAXON_JAR_LOCAL), f"-s:{xml}", f"-xsl:{xslt}", f"-o:{out}"]
    compose = ROOT / "docker-compose.yml"
    if shutil.which("docker") and compose.is_file():
        # Bind-mount is the repo; SVRL must be written under /work (not host /tmp).
        return [
            "docker",
            "compose",
            "-f",
            str(compose),
            "run",
            "--rm",
            "-T",
            "saxon",
            f"-s:{to_container(xml)}",
            f"-xsl:{to_container(xslt)}",
            f"-o:{to_container(out)}",
        ]
    raise FileNotFoundError(
        "no Saxon: install a JDK, set SAXON_JAR, or install Docker for docker compose run saxon"
    )


def find_cli() -> Path | None:
    release = ROOT / "target/release/core-invoice"
    debug = ROOT / "target/debug/core-invoice"
    if debug.is_file():
        return debug
    if release.is_file():
        return release
    return shutil.which("core-invoice") and Path(shutil.which("core-invoice"))  # type: ignore[arg-type]


# Default EN corpus: TC434 examples + credit note + guide + discount + mutant.
# Other files in refers/en16931/ubl/examples/ are named skips, not forgotten.
EN_CORE_NAMES = [
    *[f"ubl-tc434-example{i}.xml" for i in range(1, 11)],
    "ubl-tc434-creditnote1.xml",
    "guide-example1.xml",
    "guide-example2.xml",
    "guide-example3.xml",
    "sample-discount-price.xml",
]
EN_SKIP_NAMED = {
    "BIS3_Invoice_negativ.XML": "Peppol-named; not default EN XSLT compare",
    "BIS3_Invoice_positive.XML": "Peppol-named; not default EN XSLT compare",
    "FT G2G_TD01 con Allegato, Bonifico e Split Payment.xml": "Italian G2G split-payment fixture; not TC434 core",
    "issue116.xml": "ConnectingEurope regression fixture; not TC434 core",
}
PINT_MY_LHDN_SKIP = {
    "CompleteSample_LHDN.xml": "Zip LHDN-shaped sample; not IRBM Valid",
    "CompleteSample_LHDN-CreditNote.xml": "Zip LHDN-shaped sample; not IRBM Valid",
}


def default_en_files() -> list[Path]:
    base = ROOT / "refers/en16931/ubl/examples"
    out = [base / n for n in EN_CORE_NAMES if (base / n).is_file()]
    mutant = ROOT / "xtask/testdata/missing-issue-date.xml"
    if mutant.is_file():
        out.append(mutant)
    return out


def pint_my_official_xml() -> list[Path]:
    d = ROOT / "refers/pint-my-1.3.0/unpacked/trn-invoice/example"
    if not d.is_dir():
        return []
    files = sorted(d.glob("Invoice-Sample-*.xml")) + sorted(
        d.glob("CreditNote-Sample-*.xml")
    )
    return [p for p in files if p.name not in PINT_MY_LHDN_SKIP]


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
    dropped = extras_artefact_cannot_emit(
        {"PEPPOL-COMMON-R049", "BR-03"}, "en", "peppol"
    )
    assert dropped == {"PEPPOL-COMMON-R049"}, dropped
    prose = (
        "## Oracle expected-unmatched\n\n"
        "```\nBR-17\n```\n\n"
        "Prose mentions BR-99 and BR-CO-25 so a scrape would hide them.\n"
    )
    loaded = load_uncovered_text(prose)
    assert canonical("BR-17") in loaded, loaded
    assert canonical("BR-99") not in loaded, loaded
    assert canonical("BR-CO-25") not in loaded, loaded
    syn = syntax_ids({"UBL-SR-43", "BR-03", "UBL-DT-01"})
    assert syn == {"UBL-SR-43", "UBL-DT-01"}, syn
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

    try:
        saxon_cmd(
            ROOT / "refers/en16931/ubl/xslt/EN16931-UBL-validation.xslt",
            ROOT / "xtask/testdata/missing-issue-date.xml",
            ROOT / "target/svrl/_probe.xml",
        )
    except FileNotFoundError as e:
        print(f"skip: {e}")
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
        examples = ROOT / "refers/en16931/ubl/examples"
        if examples.is_dir():
            for p in sorted(examples.iterdir()):
                if p.name in EN_SKIP_NAMED:
                    print(f"skip EN example {p.name}: {EN_SKIP_NAMED[p.name]}")
        my_ex = ROOT / "refers/pint-my-1.3.0/unpacked/trn-invoice/example"
        if my_ex.is_dir():
            for name, why in PINT_MY_LHDN_SKIP.items():
                if (my_ex / name).is_file():
                    print(f"skip PINT-MY {name}: {why}")
    if not files:
        print("no XML files to compare (pass paths or fetch refers/)", file=sys.stderr)
        return 1 if require_spec() else 0

    if not xslt_en.is_file():
        print(f"missing {xslt_en}", file=sys.stderr)
        return 1 if require_spec() else 0

    svrl_dir = ROOT / "target" / "svrl"
    svrl_dir.mkdir(parents=True, exist_ok=True)
    failed = 0
    for xml in files:
        if not xml.is_file():
            print(f"missing {xml}", file=sys.stderr)
            failed += 1
            continue
        out = svrl_dir / f"{xml.stem}.svrl.xml"
        try:
            cmd = saxon_cmd(xslt_en, xml, out)
            proc = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
            if proc.returncode != 0:
                print(f"saxon failed on {xml}: {proc.stderr or proc.stdout}", file=sys.stderr)
                failed += 1
                continue
            svrl_all = parse_svrl_failed_ids(out.read_text(errors="replace"))
            syn = syntax_ids(svrl_all)
            svrl = svrl_all - syn
            ours, slug = our_fatal_ids(cli, xml, None)
            extra, missing = diff_sets(svrl, ours, mapping, uncovered)
            extra -= extras_artefact_cannot_emit(extra, "en", slug)
            print(f"{xml.name}: svrl={sorted(svrl)} ours={sorted(ours)}")
            if syn:
                print(f"  syntax ignored={sorted(syn)}")
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
    pint_my_files = pint_my_official_xml() if ("--pint-my" in argv or not argv[1:]) else []
    if my_xslt.is_file() and pint_my_files:
        for sample in pint_my_files:
            out = svrl_dir / f"pint-my-{sample.stem}.svrl.xml"
            try:
                cmd = saxon_cmd(my_xslt, sample, out)
                proc = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
                if proc.returncode != 0:
                    print(
                        f"saxon PINT-MY failed on {sample.name}: {proc.stderr or proc.stdout}",
                        file=sys.stderr,
                    )
                    failed += 1
                    continue
                svrl_all = parse_svrl_failed_ids(out.read_text(errors="replace"))
                syn = syntax_ids(svrl_all)
                svrl = svrl_all - syn
                ours, _slug = our_fatal_ids(cli, sample, "pint-my")
                extra, missing = diff_sets(svrl, ours, mapping, uncovered)
                print(f"PINT-MY {sample.name}: svrl={sorted(svrl)} ours={sorted(ours)}")
                if syn:
                    print(f"  syntax ignored={sorted(syn)}")
                if extra or missing:
                    print(
                        f"  unexpected extra={sorted(extra)} missing={sorted(missing)}",
                        file=sys.stderr,
                    )
                    failed += 1
                else:
                    print("  comparable")
            finally:
                out.unlink(missing_ok=True)

    # Sibling profiles: SST SA is not a Peppol BIS invoice.
    if sa.is_file() and xslt_en.is_file() and ("--pint-my" in argv or not argv[1:]):
        out = svrl_dir / "sa-as-en.svrl.xml"
        try:
            cmd = saxon_cmd(xslt_en, sa, out)
            proc = subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT)
            if proc.returncode != 0:
                print(f"saxon EN-on-SA failed: {proc.stderr or proc.stdout}", file=sys.stderr)
                failed += 1
            else:
                svrl_all = parse_svrl_failed_ids(out.read_text(errors="replace"))
                syn = syntax_ids(svrl_all)
                svrl = svrl_all - syn
                ours, _slug = our_fatal_ids(cli, sa, "en16931")
                extra, missing = diff_sets(svrl, ours, mapping, uncovered)
                print(f"SA as EN: svrl={sorted(svrl)} ours={sorted(ours)}")
                if syn:
                    print(f"  syntax ignored={sorted(syn)}")
                if extra or missing:
                    print(
                        f"  unexpected extra={sorted(extra)} missing={sorted(missing)}",
                        file=sys.stderr,
                    )
                    failed += 1
                elif not svrl and not ours:
                    print("  unexpected: SST SA passed EN", file=sys.stderr)
                    failed += 1
                else:
                    print("  comparable (both invalid)")
        finally:
            out.unlink(missing_ok=True)
        ours_pep, _slug = our_fatal_ids(cli, sa, "peppol")
        print(f"SA as Peppol: ours={sorted(ours_pep)}")
        if not ours_pep:
            print("  unexpected: SST SA passed Peppol", file=sys.stderr)
            failed += 1
        else:
            print("  invalid (no BIS compiled XSLT in this pin)")

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
