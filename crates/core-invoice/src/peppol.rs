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

fn r010(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R010: Buyer EndpointID. Extra_rule, not CORE. (R020 is seller.)
    if inv
        .buyer
        .electronic_address
        .as_ref()
        .map(|i| i.value.trim())
        .unwrap_or("")
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R010",
            Path::term(BtId(49)),
            "Buyer electronic address MUST be provided",
        ));
    }
}

fn r020(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R020: Seller EndpointID. Extra_rule, not CORE.
    if inv
        .seller
        .electronic_address
        .as_ref()
        .map(|i| i.value.trim())
        .unwrap_or("")
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R020",
            Path::term(BtId(34)),
            "Seller electronic address MUST be provided",
        ));
    }
}

fn r005(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(tc) = inv.tax_currency.as_ref() else {
        return;
    };
    if tc.as_str().eq_ignore_ascii_case(&inv.currency) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R005",
            Path::term(BtId(6)),
            "VAT accounting currency code MUST be different from invoice currency code when provided",
        ));
    }
}

fn r055(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(totals) = inv.totals.as_ref() else {
        return;
    };
    let Some(acct) = totals.tax_total_accounting else {
        return;
    };
    let doc = totals
        .tax_total
        .unwrap_or(crate::amount::InvoiceAmount::ZERO);
    let doc_neg = doc.raw().is_sign_negative();
    let acct_neg = acct.raw().is_sign_negative();
    if doc.is_zero() || acct.is_zero() {
        return;
    }
    if doc_neg != acct_neg {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R055",
            Path::term(BtId(111)),
            "Invoice total VAT amount and Invoice total VAT amount in accounting currency MUST have the same operational sign",
        ));
    }
}

fn r061(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(pay) = inv.payment.as_ref() else {
        return;
    };
    let is_dd = pay.means_code.as_ref().is_some_and(|c| c.as_str() == "49")
        || matches!(
            pay.means,
            Some(crate::payment::PaymentMeans::DirectDebit(_))
        );
    if !is_dd {
        return;
    }
    let mandate_ok = match &pay.means {
        Some(crate::payment::PaymentMeans::DirectDebit(d)) => {
            d.mandate.as_deref().is_some_and(|m| !m.trim().is_empty())
        }
        _ => false,
    };
    if !mandate_ok {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R061",
            Path::term(BtId(89)),
            "Mandate reference MUST be provided for direct debit",
        ));
    }
}

fn p0100(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) || inv.kind != crate::kind::DocumentKind::Invoice {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    const ALLOWED: &[&str] = &[
        "71", "80", "82", "84", "102", "218", "219", "326", "331", "380", "382", "383", "384",
        "386", "388", "393", "395", "553", "575", "623", "780", "817", "870", "875", "876", "877",
    ];
    if !ALLOWED.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-P0100",
            Path::term(BtId(3)),
            format!(
                "Invoice type code {} is not allowed for Peppol billing profile 01",
                code.as_str()
            ),
        ));
    }
}

fn vatex_pair(inv: &Invoice, report: &mut Report, vatex: &str, cat: &str, id: &'static str) {
    if !peppol_only(inv) {
        return;
    }
    for (i, row) in inv.tax_breakdown.iter().enumerate() {
        let Some(code) = row.exemption_code.as_ref() else {
            continue;
        };
        if code.as_str().eq_ignore_ascii_case(vatex)
            && !row.category.as_str().eq_ignore_ascii_case(cat)
        {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(121)),
                format!("Tax Category {cat} MUST be used when exemption reason code is {vatex}"),
            ));
        }
    }
}

fn p0104(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-G", "G", "PEPPOL-EN16931-P0104");
}
fn p0105(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-O", "O", "PEPPOL-EN16931-P0105");
}
fn p0106(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-IC", "K", "PEPPOL-EN16931-P0106");
}
fn p0107(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-AE", "AE", "PEPPOL-EN16931-P0107");
}
fn p0108(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-D", "E", "PEPPOL-EN16931-P0108");
}
fn p0109(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-F", "E", "PEPPOL-EN16931-P0109");
}
fn p0110(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-I", "E", "PEPPOL-EN16931-P0110");
}
fn p0111(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-J", "E", "PEPPOL-EN16931-P0111");
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
        "PEPPOL-EN16931-R010",
        "Buyer electronic address MUST be provided.",
        r010,
    ),
    r(
        "PEPPOL-EN16931-R020",
        "Seller electronic address MUST be provided.",
        r020,
    ),
    r(
        "PEPPOL-EN16931-R005",
        "VAT accounting currency MUST differ from invoice currency when provided.",
        r005,
    ),
    r(
        "PEPPOL-EN16931-R055",
        "BT-110 and BT-111 MUST have the same operational sign.",
        r055,
    ),
    r(
        "PEPPOL-EN16931-R061",
        "Mandate reference MUST be provided for direct debit.",
        r061,
    ),
    r(
        "PEPPOL-EN16931-P0100",
        "Invoice type code must be in the Peppol billing profile 01 list (not 389).",
        p0100,
    ),
    r(
        "PEPPOL-EN16931-P0104",
        "VATEX-EU-G requires tax category G.",
        p0104,
    ),
    r(
        "PEPPOL-EN16931-P0105",
        "VATEX-EU-O requires tax category O.",
        p0105,
    ),
    r(
        "PEPPOL-EN16931-P0106",
        "VATEX-EU-IC requires tax category K.",
        p0106,
    ),
    r(
        "PEPPOL-EN16931-P0107",
        "VATEX-EU-AE requires tax category AE.",
        p0107,
    ),
    r(
        "PEPPOL-EN16931-P0108",
        "VATEX-EU-D requires tax category E.",
        p0108,
    ),
    r(
        "PEPPOL-EN16931-P0109",
        "VATEX-EU-F requires tax category E.",
        p0109,
    ),
    r(
        "PEPPOL-EN16931-P0110",
        "VATEX-EU-I requires tax category E.",
        p0110,
    ),
    r(
        "PEPPOL-EN16931-P0111",
        "VATEX-EU-J requires tax category E.",
        p0111,
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
        inv.seller.electronic_address = Some(Identifier::schemed("1234567890128", "0088"));
        inv.buyer.electronic_address = Some(Identifier::schemed("1234567890129", "0088"));
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
    fn r010_buyer_endpoint_not_on_en() {
        let mut inv = peppol();
        inv.buyer.electronic_address = None;
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R010"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R010"),
            "{report}"
        );
    }

    #[test]
    fn p0100_forbids_389_on_peppol_not_en() {
        let mut inv = peppol();
        inv.type_code = Some(Code::new("389"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-P0100"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-P0100"),
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
