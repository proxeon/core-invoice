#!/usr/bin/env python3
"""Extract UBL/CII syntax prohibitions from CEN preprocessed Schematron.

Context is half the rule: `not(cbc:UUID)` on `/ubl:Invoice` is not a blanket
ban on `cbc:UUID`. Do not drop the context. Generated files are MIT OR Apache-2.0
(code points / paths only).
"""
from __future__ import annotations

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NS = {"sch": "http://purl.oclc.org/dsdl/schematron"}
ARTEFACTS = {
    "ubl": ROOT
    / "refers/en16931/ubl/schematron/preprocessed/EN16931-UBL-validation-preprocessed.sch",
    "cii": ROOT
    / "refers/en16931/cii/schematron/preprocessed/EN16931-CII-validation-preprocessed.sch",
}
OUT = {
    "ubl": ROOT / "crates/core-invoice-formats/src/prohibitions_ubl.rs",
    "cii": ROOT / "crates/core-invoice-formats/src/prohibitions_cii.rs",
}

ATTR_LOCAL = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def is_syntax_rule(rid: str) -> bool:
    parts = rid.split("-")
    if len(parts) != 3:
        return False
    prefix, kind, num = parts
    return (
        prefix.isascii()
        and prefix.isupper()
        and kind in {"CR", "SR", "DT"}
        and num.isdigit()
    )


def is_name(seg: str) -> bool:
    halves = seg.split(":")
    if len(halves) != 2:
        return False
    prefix, local = halves
    if not prefix or not prefix[0].isalpha():
        return False
    if not all(c.isalnum() or c in "_.-" for c in prefix):
        return False
    if not local or not all(c.isalnum() or c in "_.-" for c in local):
        return False
    return True


def is_element_chain(s: str) -> bool:
    if not s:
        return False
    return all(is_name(seg) for seg in s.split("/"))


def is_attr_local(s: str) -> bool:
    return bool(ATTR_LOCAL.fullmatch(s))


def is_usable_context(branch: str) -> bool:
    if not branch:
        return False
    if branch.startswith("//"):
        body = branch[2:]
        return bool(body) and not body.startswith("/") and is_element_chain(body)
    if branch.startswith("/"):
        body = branch[1:]
        return bool(body) and is_element_chain(body)
    return is_element_chain(branch)


def strip_not(test: str) -> str | None:
    """Unwrap a single balanced `not(…)` that is the entire test.

    `not(x) or y` is not a prohibition of `x` and is not counted.
    """
    test = test.strip()
    if not test.startswith("not("):
        return None
    depth = 0
    for i, ch in enumerate(test):
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                if i != len(test) - 1:
                    return None
                return test[4:i].strip()
            if depth < 0:
                return None
    return None


def expand_alternation(inner: str) -> list[str] | None:
    if is_element_chain(inner):
        return [inner]
    if inner.startswith("(") and ")" in inner:
        head, _, rest = inner[1:].partition(")")
        if "(" in head:
            return None
        tail = rest.strip()
        if tail.startswith("/"):
            suffix = tail[1:]
            if not is_element_chain(suffix):
                return None
        elif tail:
            return None
        else:
            suffix = None
        out = []
        for branch in (b.strip() for b in head.split("|")):
            if not is_element_chain(branch):
                return None
            out.append(f"{branch}/{suffix}" if suffix else branch)
        return out or None
    if "|" in inner:
        branches = [b.strip() for b in inner.split("|")]
        if all(is_element_chain(b) for b in branches):
            return branches
    return None


def parse_inner(inner: str):
    """Classify a `not(…)` inner path.

    Returns one of:
      ("doc_attr", attr)
      ("ctx_attr", element_chain, attr, floating)
      ("elem", [(override_context_or_None, relative), ...], floating)
      None if the inner needs an XPath engine or is a mutual exclusion.
    """
    inner = inner.strip()
    if not inner:
        return None

    m = re.fullmatch(r"//@([A-Za-z_][A-Za-z0-9_]*)", inner)
    if m:
        return ("doc_attr", m.group(1))

    floating = inner.startswith("//")
    rest = inner[2:] if floating else inner
    if floating and (not rest or rest.startswith("/")):
        return None

    if re.search(r"[\[\]\*]", rest) or "ends-with" in rest or "::" in rest:
        return None

    if rest.startswith("@"):
        if floating:
            return None
        attr = rest[1:]
        if not is_attr_local(attr):
            return None
        return ("ctx_attr", "", attr, False)

    if "/@" in rest:
        elem, attr = rest.rsplit("/@", 1)
        if not is_element_chain(elem) or not is_attr_local(attr):
            return None
        return ("ctx_attr", elem, attr, floating)

    if is_element_chain(rest):
        segs = rest.split("/")
        if floating:
            if len(segs) == 1:
                return ("elem", [(f"//{rest}", "")], True)
            parent = "/".join(segs[:-1])
            return ("elem", [(f"//{parent}", segs[-1])], True)
        return ("elem", [(None, rest)], False)

    if floating:
        return None
    branches = expand_alternation(inner)
    if branches:
        return ("elem", [(None, b) for b in branches], False)
    return None


def rust_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def context_branches(context: str) -> list[str]:
    return [b.strip() for b in context.split("|")]


def join_path(prefix: str, rel: str) -> str:
    if not rel:
        return prefix
    if prefix.endswith("/"):
        return prefix + rel
    return f"{prefix}/{rel}"


def extract(
    syntax: str,
) -> tuple[list[str], list[str], list[str], list[str], int, int]:
    path = ARTEFACTS[syntax]
    tree = ET.parse(path)
    root = tree.getroot()
    paths: set[str] = set()
    attrs: set[str] = set()
    attr_paths: set[str] = set()
    unextracted_ids: list[str] = []
    assertions = 0
    for rule in root.findall(".//sch:rule", NS):
        context = (rule.get("context") or "").strip()
        for assertion in rule.findall("sch:assert", NS):
            rid = assertion.get("id")
            if not rid or not is_syntax_rule(rid):
                continue
            test = (assertion.get("test") or "").strip()
            inner = strip_not(test)
            if inner is None:
                continue
            assertions += 1
            parsed = parse_inner(inner)
            if parsed is None:
                unextracted_ids.append(rid)
                continue
            kind = parsed[0]
            if kind == "doc_attr":
                attrs.add(f'("{rust_escape(rid)}", "{rust_escape(parsed[1])}"),')
                continue
            if kind == "ctx_attr":
                _, elem, attr, floating = parsed
                if floating:
                    ctx = f"//{elem}"
                    attr_paths.add(
                        f'("{rust_escape(rid)}", "{rust_escape(ctx)}", "{rust_escape(attr)}"),'
                    )
                    continue
                branches = context_branches(context)
                if not all(is_usable_context(b) for b in branches):
                    unextracted_ids.append(rid)
                    continue
                for b in branches:
                    ctx = join_path(b, elem)
                    attr_paths.add(
                        f'("{rust_escape(rid)}", "{rust_escape(ctx)}", "{rust_escape(attr)}"),'
                    )
                continue
            _, rows, floating = parsed
            if floating:
                for ctx, rel in rows:
                    assert ctx is not None
                    paths.add(
                        f'("{rust_escape(rid)}", "{rust_escape(ctx)}", "{rust_escape(rel)}"),'
                    )
                continue
            branches = context_branches(context)
            if not all(is_usable_context(b) for b in branches):
                unextracted_ids.append(rid)
                continue
            for b in branches:
                for _, rel in rows:
                    paths.add(
                        f'("{rust_escape(rid)}", "{rust_escape(b)}", "{rust_escape(rel)}"),'
                    )
    if not paths and not attrs and not attr_paths:
        raise SystemExit(f"{path}: no prohibitions extracted")
    unique_ids = sorted(set(unextracted_ids))
    id_rows = [f'"{rust_escape(i)}",' for i in unique_ids]
    return (
        sorted(paths),
        sorted(attrs),
        sorted(attr_paths),
        id_rows,
        len(unextracted_ids),
        assertions,
    )


def render(
    syntax: str,
    paths: list[str],
    attrs: list[str],
    attr_paths: list[str],
    unextracted_ids: list[str],
    unextracted: int,
    assertions: int,
) -> str:
    label = syntax.upper()
    checked = assertions - unextracted
    body_p = "".join(f"    {l}\n" for l in paths)
    body_a = "".join(f"    {l}\n" for l in attrs)
    body_ap = "".join(f"    {l}\n" for l in attr_paths)
    body_u = "".join(f"    {l}\n" for l in unextracted_ids)
    return f"""//! {label} prohibitions, extracted from CEN's preprocessed Schematron.
//!
//! **Generated by `python3 xtask/gen_prohibitions.py`. Do not edit.**
//!
//! Each entry is `(rule, context, relative path)`. `not(x)` forbids `x` only
//! under that context, not everywhere. Source is the preprocessed artefact
//! (resolved `rule/@context`).
//!
//! {checked} of {assertions} `not(…)` assertions; {len(paths)} element rows,
//! {len(attrs)} document-wide (`//@attr`) rows, {len(attr_paths)} contextual
//! attribute rows. [`UNEXTRACTED`] = {unextracted} remaining are predicates,
//! `ends-with` / `*` wildcards, or mutual exclusions (`not(A and B)`).
//! `not(elem/@attr)` and `not(//elem)` are extracted with their context.

/// `(rule id, context, forbidden path relative to that context)`.
///
/// Empty relative means the context node itself (`//cac:FinancialInstitution`).
pub static FORBIDDEN_PATHS: &[(&str, &str, &str)] = &[
{body_p}];

/// `(rule id, attribute local name)` forbidden anywhere (`//@attr`).
pub static FORBIDDEN_ATTRIBUTES: &[(&str, &str)] = &[
{body_a}];

/// `(rule id, context of the element, forbidden attribute local name)`.
///
/// Context is kept: `cbc:CompanyID/@schemeID` is not `//@schemeID`.
pub static FORBIDDEN_ATTRIBUTE_PATHS: &[(&str, &str, &str)] = &[
{body_ap}];

/// Rule ids whose `not(…)` test was not extracted.
pub static UNEXTRACTED_IDS: &[&str] = &[
{body_u}];

pub const TOTAL_PARAMS: usize = {assertions};
pub const UNEXTRACTED: usize = {unextracted};
"""


def self_test() -> None:
    assert strip_not("not(cbc:UUID)") == "cbc:UUID"
    assert strip_not("not(ram:GlobalID) or (ram:GlobalID/@schemeID)") is None
    assert strip_not("not(cbc:UBLVersionID) or cbc:UBLVersionID = '2.1'") is None
    assert strip_not("not(ram:A and ram:B)") == "ram:A and ram:B"
    nested = (
        "not(//cac:AdditionalDocumentReference[cbc:DocumentTypeCode != '130' "
        "or not(cbc:DocumentTypeCode)]/cbc:ID/@schemeID)"
    )
    inner = strip_not(nested)
    assert inner is not None and inner.startswith("//cac:AdditionalDocumentReference")
    assert parse_inner(inner) is None

    assert expand_alternation("cbc:UUID") == ["cbc:UUID"]
    assert expand_alternation(
        "(cac:InvoiceLine|cac:CreditNoteLine)/cac:SubInvoiceLine"
    ) == [
        "cac:InvoiceLine/cac:SubInvoiceLine",
        "cac:CreditNoteLine/cac:SubInvoiceLine",
    ]

    assert parse_inner("cbc:UUID") == ("elem", [(None, "cbc:UUID")], False)
    assert parse_inner("cbc:CustomizationID/@schemeID") == (
        "ctx_attr",
        "cbc:CustomizationID",
        "schemeID",
        False,
    )
    assert parse_inner("//cac:FinancialInstitution") == (
        "elem",
        [("//cac:FinancialInstitution", "")],
        True,
    )
    assert parse_inner("//cac:PaymentMeans/cac:PayerFinancialAccount") == (
        "elem",
        [("//cac:PaymentMeans", "cac:PayerFinancialAccount")],
        True,
    )
    assert parse_inner("//cac:PartyTaxScheme/cbc:CompanyID/@schemeID") == (
        "ctx_attr",
        "cac:PartyTaxScheme/cbc:CompanyID",
        "schemeID",
        True,
    )
    assert parse_inner("@schemeName") == ("ctx_attr", "", "schemeName", False)
    assert parse_inner("//@unitCodeListID") == ("doc_attr", "unitCodeListID")
    assert parse_inner("cac:Party[cbc:Name]") is None
    assert parse_inner("//*[ends-with(name(), 'Amount')]") is None
    assert parse_inner("ram:SellerTradeParty/ram:DefinedTradeContact/ram:PersonName and ram:SellerTradeParty/ram:DefinedTradeContact/ram:DepartmentName") is None
    assert is_usable_context("/ubl:Invoice")
    assert is_usable_context("//ram:TypeCode")
    assert not is_usable_context("//ram:*[ends-with(name(), 'TradeTax')]")
    assert not is_usable_context("//*[ends-with(name(), 'DocumentContextParameter')]")


def main() -> None:
    self_test()
    if "--self-test" in sys.argv:
        print("self-test ok")
        return
    missing = [p for p in ARTEFACTS.values() if not p.is_file()]
    if missing:
        print("missing preprocessed Schematron:", *missing, file=sys.stderr)
        sys.exit(1)
    for syntax in ("ubl", "cii"):
        paths, attrs, attr_paths, unextracted_ids, unextracted, assertions = extract(
            syntax
        )
        OUT[syntax].write_text(
            render(
                syntax,
                paths,
                attrs,
                attr_paths,
                unextracted_ids,
                unextracted,
                assertions,
            ),
            encoding="utf-8",
        )
        ids = [row.strip().strip(",").strip('"') for row in unextracted_ids]
        print(
            f"{syntax}: {len(paths)} paths, {len(attrs)} doc-attrs, "
            f"{len(attr_paths)} ctx-attrs, "
            f"{assertions - unextracted}/{assertions} extracted, "
            f"unextracted={unextracted} ids={ids} "
            f"→ {OUT[syntax].relative_to(ROOT)}"
        )


if __name__ == "__main__":
    main()
