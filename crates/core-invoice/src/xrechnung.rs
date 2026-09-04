//! XRechnung extra rules. Optional Cargo feature, never default, never CORE.
//!
//! XRechnung is a German CIUS of EN 16931. BT-24 still looks up as
//! [`crate::profile::Profile::En16931`]. These rules overlay only when the
//! `xrechnung` feature is on **and** the document claims an XRechnung
//! specification identifier.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;
use crate::tax::TaxSystem;

/// KoSIT / xeinkauf XRechnung specification identifiers (any 1.x/2.x/3.x).
pub fn claimed(inv: &Invoice) -> bool {
    inv.specification_id
        .as_deref()
        .is_some_and(is_xrechnung_spec)
}

pub fn is_xrechnung_spec(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    id.contains("xrechnung") || id.contains("xeinkauf.de")
}

fn br_de_15(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if inv
        .buyer_reference
        .as_ref()
        .map(|r| r.as_str().trim().is_empty())
        .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-DE-15",
            Path::term(BtId(10)),
            "XRechnung: Buyer reference (BT-10) shall be present",
        ));
    }
}

fn taxed_vat(inv: &Invoice) -> bool {
    let hit = |code: &str| {
        let c = code.trim();
        !c.is_empty() && !c.eq_ignore_ascii_case("O") && !c.eq_ignore_ascii_case("B")
    };
    inv.lines
        .iter()
        .any(|l| l.tax.system == TaxSystem::Vat && hit(&l.tax.code))
        || inv.tax_breakdown.iter().any(|e| hit(e.category.as_str()))
}

fn br_de_16(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) || !taxed_vat(inv) {
        return;
    }
    let seller_id = inv.seller.vat_identifier.is_some() || inv.seller.tax_registration.is_some();
    let rep = inv
        .tax_representative
        .as_ref()
        .is_some_and(|r| r.vat_identifier.is_some());
    if !seller_id && !rep {
        report.push(Finding::fatal(
            "BR-DE-16",
            Path::group_term(Group::Seller, BtId(31)),
            "XRechnung: BT-31, BT-32 or BT-63 shall be present when VAT category is not O/B",
        ));
    }
}

const fn r(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Fatal,
        text,
        source: Source::Crate,
        eval,
    }
}

/// Extra rules. Not CORE. Remaining KoSIT BR-DE-* stay in UNCOVERED until evaluated.
pub static RULES: &[Rule] = &[
    r(
        "BR-DE-15",
        "XRechnung: Buyer reference (BT-10) shall be present.",
        br_de_15,
    ),
    r(
        "BR-DE-16",
        "XRechnung: seller VAT/tax id or tax representative when category is not O/B.",
        br_de_16,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Code;
    use crate::identifier::DocumentReference;
    use crate::invoice::{Invoice, Line, Party};
    use crate::profile::Profile;
    use crate::tax::TaxCategory;
    use crate::validate::validate;
    use rust_decimal::Decimal;

    fn xr() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "DE"),
        );
        inv.specification_id =
            Some("urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0".into());
        inv.lines = vec![Line::new(
            "1",
            "A",
            crate::amount::Amount::parse("10.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        inv
    }

    #[test]
    fn detect_kosit_urn() {
        assert!(is_xrechnung_spec(
            "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0"
        ));
        assert!(!is_xrechnung_spec("urn:cen.eu:en16931:2017"));
    }

    #[test]
    fn br_de_15_requires_buyer_reference() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-15"),
            "{report}"
        );
        let mut inv = inv;
        inv.buyer_reference = Some(DocumentReference::new("PO-1"));
        inv.seller.vat_identifier = Some(crate::identifier::Identifier::new("DE123"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-15"),
            "{report}"
        );
    }

    #[test]
    fn br_de_16_requires_seller_tax_id() {
        let mut inv = xr();
        inv.buyer_reference = Some(DocumentReference::new("PO-1"));
        inv.type_code = Some(Code::new("380"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-16"),
            "{report}"
        );
    }

    #[test]
    fn en_core_does_not_run_br_de() {
        let mut inv = xr();
        inv.specification_id = Some("urn:cen.eu:en16931:2017".into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "BR-DE-15" && f.id != "BR-DE-16"),
            "{report}"
        );
    }
}
