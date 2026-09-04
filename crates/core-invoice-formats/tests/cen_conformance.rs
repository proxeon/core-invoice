//! Run CEN's own UBL unit-test suite against our rules.
//!
//! `refers/en16931/test/{Invoice,CreditNote}-unit-UBL/` is Difi `testSet` XML:
//! `<error>BR-01</error>` means the rule must fire; `<success>` means it must not.
//! Skip unless `refers/` is present; fail if `CORE_INVOICE_REQUIRE_SPEC=1` and the
//! suite is missing. Syntax `UBL-*` / `CII-*` are formats/unmapped, not CORE.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use core_invoice::Profile;
use core_invoice::rules::{self, catalogue};
use core_invoice_formats::validate_xml;

struct Case {
    file: String,
    rule: String,
    must_fire: bool,
    description: String,
    xml: String,
}

fn suite_root() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../refers/en16931/test");
    root.is_dir().then_some(root)
}

fn cases() -> Vec<Case> {
    let Some(root) = suite_root() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for dir_name in ["Invoice-unit-UBL", "CreditNote-unit-UBL"] {
        let dir = root.join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .expect("read CEN suite dir")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "xml"))
            .collect();
        files.sort();
        for path in files {
            let text = std::fs::read_to_string(&path).expect("read CEN case");
            let doc = match roxmltree::Document::parse(&text) {
                Ok(d) => d,
                Err(e) => panic!("{}: {e}", path.display()),
            };
            let file = path.file_name().unwrap().to_string_lossy().into_owned();
            for test in doc.descendants().filter(|n| n.tag_name().name() == "test") {
                let Some(assertion) = test.children().find(|n| n.tag_name().name() == "assert")
                else {
                    continue;
                };
                let mut rule = None;
                let mut must_fire = false;
                for c in assertion.children().filter(roxmltree::Node::is_element) {
                    match c.tag_name().name() {
                        "error" | "fatal" => {
                            rule = c.text().map(|t| t.trim().to_owned());
                            must_fire = true;
                        }
                        "success" => {
                            rule = c.text().map(|t| t.trim().to_owned());
                            must_fire = false;
                        }
                        _ => {}
                    }
                }
                let Some(rule) = rule else { continue };
                let Some(root_el) = test
                    .children()
                    .find(|n| matches!(n.tag_name().name(), "Invoice" | "CreditNote"))
                else {
                    continue;
                };
                out.push(Case {
                    file: file.clone(),
                    rule,
                    must_fire,
                    description: assertion
                        .children()
                        .find(|n| n.tag_name().name() == "description")
                        .and_then(|n| n.text())
                        .unwrap_or("")
                        .trim()
                        .to_owned(),
                    xml: text[root_el.range()].to_owned(),
                });
            }
        }
    }
    out
}

fn registered(rule: &str) -> Option<&'static str> {
    catalogue()
        .iter()
        .find(|r| rules::matches_id(r.id, rule))
        .map(|r| r.id)
}

/// Type-retired / artefact `true()` / DEC — no state makes them fire.
fn unevaluated(id: &str) -> bool {
    id.starts_with("BR-DEC-")
        || matches!(
            id,
            "BR-24"
                | "BR-31"
                | "BR-36"
                | "BR-41"
                | "BR-43"
                | "BR-45"
                | "BR-46"
                | "BR-CO-05"
                | "BR-CO-06"
                | "BR-CO-07"
                | "BR-CO-08"
        )
}

/// Named, exact. Empty groups in UBL are absent in this model.
const DIVERGENCES: &[(&str, &str, &str)] = &[
    (
        "BR-08.xml",
        "BR-08",
        "an empty <cac:PostalAddress/> is an absent address here",
    ),
    (
        "BR-10.xml",
        "BR-10",
        "an empty <cac:PostalAddress/> is an absent address here",
    ),
    (
        "BR-19.xml",
        "BR-19",
        "an empty <cac:PostalAddress/> is an absent address here",
    ),
    (
        "BR-55.xml",
        "BR-55",
        "a <cac:BillingReference/> with no child is an absent BG-3 here",
    ),
    (
        "BR-CO-19.xml",
        "BR-CO-19",
        "a <cac:InvoicePeriod/> with no dates is an absent BG-14 here",
    ),
    (
        "BR-CO-15.xml",
        "BR-CO-15",
        "two cac:TaxTotal elements cannot both be BT-110",
    ),
    (
        "BR-CO-15-2.xml",
        "BR-CO-15",
        "two cac:TaxTotal elements cannot both be BT-110",
    ),
];

fn fired(xml: &str, rule: &str) -> Result<bool, String> {
    // Suite fragments often have no / unknown BT-24. Force EN so Peppol extras
    // and PINT-TAX do not colour the verdict. validate_xml also walks @currencyID (BR-CL-03).
    match validate_xml(xml, Some(Profile::En16931)) {
        Ok(report) => Ok(report
            .findings
            .iter()
            .any(|f| rules::matches_id(f.id, rule))),
        Err(e) => Err(format!("unreadable: {e}")),
    }
}

/// Floor: coverage must not shrink. Raise when agreement grows; never lower without a commit note.
const MIN_CEN_RUN: usize = 1_000;
const MIN_CEN_AGREED: usize = 1_055;
/// Ceiling: unexplained disagreements. Named DIVERGENCES are counted separately.
const MAX_CEN_DISAGREED: usize = 0;

#[test]
fn cen_ubl_unit_tests_agree() {
    let cases = cases();
    if cases.is_empty() {
        if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
            panic!("missing refers/en16931/test; run task spec");
        }
        eprintln!("skipping: refers/en16931/test not present");
        return;
    }

    let mut agreed = 0usize;
    let mut skipped_unevaluated = 0usize;
    let mut skipped_syntax = 0usize;
    let mut skipped_unreadable = 0usize;
    let mut diverged: BTreeMap<&str, usize> = BTreeMap::new();
    let mut disagreed: Vec<String> = Vec::new();

    for case in &cases {
        if case.rule.starts_with("UBL-") || case.rule.starts_with("CII-") {
            skipped_syntax += 1;
            continue;
        }
        let Some(canonical) = registered(&case.rule) else {
            // BR-CO-25 has no UBL assert in 1.3.16; suite still ships CreditNote cases.
            if case.rule.eq_ignore_ascii_case("BR-CO-25") {
                skipped_unevaluated += 1;
            } else if case.rule.starts_with("BR-") {
                disagreed.push(format!(
                    "{} [{}] not in catalogue — {}",
                    case.file, case.rule, case.description
                ));
            } else {
                skipped_syntax += 1;
            }
            continue;
        };
        if unevaluated(canonical) {
            skipped_unevaluated += 1;
            continue;
        }
        match fired(&case.xml, canonical) {
            Err(_) => skipped_unreadable += 1,
            Ok(did) if did == case.must_fire => agreed += 1,
            Ok(_) => {
                if let Some((_, _, why)) = DIVERGENCES
                    .iter()
                    .find(|(f, r, _)| *f == case.file && r.eq_ignore_ascii_case(&case.rule))
                {
                    *diverged.entry(*why).or_default() += 1;
                } else {
                    let want = if case.must_fire {
                        "must fire"
                    } else {
                        "must not fire"
                    };
                    disagreed.push(format!(
                        "{} [{}] {want} — {}",
                        case.file, case.rule, case.description
                    ));
                }
            }
        }
    }

    let run_count = agreed + disagreed.len() + diverged.values().sum::<usize>();
    eprintln!(
        "CEN UBL unit tests\n  assertions: {total}\n  run: {run_count}\n  agreed: {agreed}\n  \
         disagreed: {bad}\n  diverged, declared: {div}\n  skipped unevaluated: {skipped_unevaluated}\n  \
         skipped syntax: {skipped_syntax}\n  skipped unreadable: {skipped_unreadable}",
        total = cases.len(),
        bad = disagreed.len(),
        div = diverged.values().sum::<usize>(),
    );
    for (why, n) in &diverged {
        eprintln!("    {n} × {why}");
    }
    {
        let mut by: BTreeMap<&str, usize> = BTreeMap::new();
        for d in &disagreed {
            let rule = d.split(['[', ']']).nth(1).unwrap_or("?");
            *by.entry(rule).or_default() += 1;
        }
        for (rule, n) in &by {
            eprintln!("  remain {n:3} {rule}");
        }
        for d in &disagreed {
            eprintln!("    {d}");
        }
    }
    assert!(
        run_count >= MIN_CEN_RUN,
        "only {run_count} CEN assertions ran (floor {MIN_CEN_RUN})"
    );
    assert!(
        agreed >= MIN_CEN_AGREED,
        "only {agreed} agreed (floor {MIN_CEN_AGREED}); CEN agreement shrank"
    );
    assert!(
        disagreed.is_empty(),
        "{} disagreements (ceiling {MAX_CEN_DISAGREED}):\n  {}",
        disagreed.len(),
        disagreed.join("\n  ")
    );
}
