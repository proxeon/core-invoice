//! Peppol BIS Billing 3.0 extra_rules. Not CORE, not inherited by PINT.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::profile::Profile;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;
use rust_decimal::Decimal;

fn peppol_only(inv: &Invoice) -> bool {
    inv.profile == Profile::PeppolBis3
}

fn r001(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    if inv
        .business_process
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R001",
            Path::term(BtId(23)),
            "Business process type (BT-23 / ProfileID) shall be present",
        ));
    }
}

fn r007(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(id) = inv.business_process.as_deref() else {
        return;
    };
    // Profile 01. Profile 02 is Later.
    if !id.starts_with("urn:fdc:peppol.eu:2017:poacc:billing:") || !id.ends_with(":1.0") {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R007",
            Path::term(BtId(23)),
            format!("BT-23 {id} is not urn:fdc:peppol.eu:2017:poacc:billing:NN:1.0"),
        ));
    }
}

fn r004(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(id) = inv.specification_id.as_deref() else {
        return;
    };
    if !id.starts_with(Profile::PEPPOL_BIS3_PREFIX) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R004",
            Path::term(BtId(24)),
            "BT-24 shall start with the official Peppol BIS Billing 3.0 identifier",
        ));
    }
}

fn r003(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R003: BT-10 or BT-13, not BG-3.
    let has_buyer_ref = inv
        .buyer_reference
        .as_ref()
        .is_some_and(|r| !r.as_str().trim().is_empty());
    let has_order = inv
        .purchase_order
        .as_ref()
        .is_some_and(|r| !r.as_str().trim().is_empty());
    if !has_buyer_ref && !has_order {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R003",
            Path::term(BtId(10)),
            "Buyer reference (BT-10) or order reference (BT-13) shall be present",
        ));
    }
}

/// ±0.02 **inclusive**. A rule instance, not a crate constant. R046 is exact.
fn r120(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let two_cents = Decimal::new(2, 2);
    for (i, line) in inv.lines.iter().enumerate() {
        let (Some(qty), Some(price)) = (line.quantity, line.price.as_ref()) else {
            continue;
        };
        let base = price
            .base_qty
            .map(|q| q.raw())
            .filter(|d| !d.is_zero())
            .unwrap_or(Decimal::ONE);
        let Some(mut expected) = qty
            .raw()
            .checked_mul(price.net.raw())
            .and_then(|v| v.checked_div(base))
        else {
            continue;
        };
        for c in &line.charges {
            let Some(v) = expected.checked_add(c.amount.raw()) else {
                continue;
            };
            expected = v;
        }
        for a in &line.allowances {
            let Some(v) = expected.checked_sub(a.amount.raw()) else {
                continue;
            };
            expected = v;
        }
        let delta = (expected - line.net.raw()).abs();
        if delta > two_cents {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R120",
                Path::at_term(Group::Line, i, BtId(131)),
                format!(
                    "BT-131 {} differs from qty×price/base by {delta} (slack ±0.02 inclusive)",
                    line.net
                ),
            ));
        }
    }
}

fn r046(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(price) = line.price.as_ref() else {
            continue;
        };
        let Some(gross) = price.gross else {
            continue;
        };
        let discount = price
            .discount
            .unwrap_or(crate::amount::UnitPriceAmount::ZERO);
        let Some(expected) = gross.raw().checked_sub(discount.raw()) else {
            continue;
        };
        if expected != price.net.raw() {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R046",
                Path::at_term(Group::Line, i, BtId(146)),
                "net price = gross − discount, exact (not R120 slack)",
            ));
        }
    }
}

fn r040(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let two_cents = Decimal::new(2, 2);
    for (i, a) in inv
        .document_allowances
        .iter()
        .chain(inv.document_charges.iter())
        .enumerate()
    {
        let (Some(base), Some(pct)) = (a.base, a.percent) else {
            continue;
        };
        let Some(expected) = base
            .raw()
            .checked_mul(pct.as_percent())
            .map(|v| v / Decimal::ONE_HUNDRED)
        else {
            continue;
        };
        if (expected - a.amount.raw()).abs() > two_cents {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R040",
                Path::at_term(Group::DocumentAllowance, i, BtId(92)),
                format!(
                    "allowance/charge amount {} differs from base×percent/100 by more than 0.02",
                    a.amount
                ),
            ));
        }
    }
}

fn r121(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(price) = line.price.as_ref() else {
            continue;
        };
        if let Some(q) = price.base_qty
            && (!q.raw().is_sign_positive() || q.raw().is_zero())
        {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R121",
                Path::at_term(Group::Line, i, BtId(149)),
                "base quantity shall be greater than zero",
            ));
        }
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

pub static RULES: &[Rule] = &[
    r("PEPPOL-EN16931-R001", "BT-23 shall be present.", r001),
    r(
        "PEPPOL-EN16931-R007",
        "BT-23 shall match urn:fdc:peppol.eu:2017:poacc:billing:NN:1.0.",
        r007,
    ),
    r(
        "PEPPOL-EN16931-R004",
        "BT-24 shall start with the official Peppol BIS Billing 3.0 id.",
        r004,
    ),
    r(
        "PEPPOL-EN16931-R003",
        "BT-10 or BT-13 shall be present.",
        r003,
    ),
    r(
        "PEPPOL-EN16931-R120",
        "Line net ≈ qty × (price / base qty), slack ±0.02 inclusive.",
        r120,
    ),
    r(
        "PEPPOL-EN16931-R040",
        "Allowance/charge amount ≈ base × percent/100, slack ±0.02 inclusive.",
        r040,
    ),
    r(
        "PEPPOL-EN16931-R046",
        "Net price = gross − discount, exact.",
        r046,
    ),
    r(
        "PEPPOL-EN16931-R121",
        "Base quantity shall be greater than zero.",
        r121,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::InvoiceAmount;
    use crate::code::Code;
    use crate::date::Date;
    use crate::identifier::Identifier;
    use crate::invoice::{Line, Party, Price};
    use crate::numeric::Quantity;
    use crate::reconcile::reconcile;
    use crate::tax::TaxCategory;
    use crate::validate;
    use rust_decimal::Decimal;

    fn peppol() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::PeppolBis3,
            "EU-1",
            "EUR",
            {
                let mut p = Party::new("S", "DE");
                p.vat_identifier = Some(Identifier::new("DE123456789"));
                p
            },
            {
                let mut b = Party::new("B", "FR");
                b.vat_identifier = Some(Identifier::new("FR12345678901"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
        inv.buyer_reference = Some(crate::identifier::DocumentReference::new("PO-1"));
        inv.lines = vec![Line::new(
            "1",
            "A",
            InvoiceAmount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn r120_fails_at_three_cents_not_on_en16931() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.03").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R120"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn r120_passes_at_two_cents() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.02").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn r003_bt13_not_preceding() {
        let mut inv = peppol();
        inv.buyer_reference = None;
        inv.preceding.push(crate::invoice::PrecedingInvoice {
            reference: crate::identifier::DocumentReference::new("INV-OLD"),
            issue_date: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R003"),
            "BG-3 alone must not satisfy R003: {report}"
        );
        inv.purchase_order = Some(crate::identifier::DocumentReference::new("PO-13"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R003"),
            "{report}"
        );
    }

    #[test]
    fn r120_includes_line_charges_minus_allowances() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        inv.lines[0]
            .charges
            .push(crate::invoice::LineAllowanceCharge {
                amount: InvoiceAmount::parse("5.00").unwrap(),
                base: None,
                percent: None,
                reason: None,
                reason_code: None,
            });
        inv.lines[0].net = InvoiceAmount::parse("105.00").unwrap();
        let _ = reconcile(&mut inv);
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn pint_my_does_not_run_peppol_r001() {
        let mut inv = peppol();
        inv.profile = Profile::PintMy;
        inv.specification_id = Some(Profile::PintMy.specification_id().into());
        inv.business_process = None;
        inv.seller.legal_registration = Some(Identifier::new("2023010000001"));
        inv.seller.tax_registration = Some(Identifier::new("C12345678901"));
        inv.buyer.legal_registration = Some(Identifier::new("1999010000001"));
        inv.lines[0].tax = TaxCategory::sst("SA", Decimal::from(10));
        let _ = reconcile(&mut inv);
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| !f.id.starts_with("PEPPOL-EN16931")),
            "{report}"
        );
    }
}
