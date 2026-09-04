//! XRechnung extra rules. Optional Cargo feature, never default, never CORE.
//!
//! XRechnung is a German CIUS of EN 16931. BT-24 still looks up as
//! [`crate::profile::Profile::En16931`]. These rules overlay only when the
//! `xrechnung` feature is on **and** the document claims an XRechnung
//! specification identifier.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::payment::PaymentMeans;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;

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
    const CODES: &[&str] = &["S", "Z", "E", "AE", "K", "G", "L", "M"];
    let hit = |code: &str| CODES.iter().any(|c| c.eq_ignore_ascii_case(code.trim()));
    inv.lines.iter().any(|l| hit(&l.tax.code))
        || inv
            .document_allowances
            .iter()
            .any(|a| a.tax.as_ref().is_some_and(|t| hit(&t.code)))
        || inv
            .document_charges
            .iter()
            .any(|a| a.tax.as_ref().is_some_and(|t| hit(&t.code)))
}

fn br_de_16(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) || !taxed_vat(inv) {
        return;
    }
    let seller_id = inv.seller.vat_identifier.is_some() || inv.seller.tax_registration.is_some();
    let rep = inv.tax_representative.is_some();
    if !seller_id && !rep {
        report.push(Finding::fatal(
            "BR-DE-16",
            Path::group_term(Group::Seller, BtId(31)),
            "XRechnung: BT-31, BT-32 or tax representative shall be present for listed VAT categories",
        ));
    }
}

fn blank(s: Option<&str>) -> bool {
    s.map(|t| t.trim().is_empty()).unwrap_or(true)
}

fn br_de_1(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if inv.payment.is_none() {
        report.push(Finding::fatal(
            "BR-DE-1",
            Path::group(Group::Payment),
            "XRechnung: Payment instructions (BG-16) shall be present",
        ));
    }
}

fn br_de_2(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let empty = inv.seller.contact.as_ref().is_none_or(|c| {
        blank(c.point.as_deref()) && blank(c.phone.as_deref()) && blank(c.email.as_deref())
    });
    if empty {
        report.push(Finding::fatal(
            "BR-DE-2",
            Path::group_term(Group::Seller, BtId(41)),
            "XRechnung: Seller contact (BG-6) shall be present",
        ));
    }
}

fn br_de_3(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(inv.seller.address.as_ref().and_then(|a| a.city.as_deref())) {
        report.push(Finding::fatal(
            "BR-DE-3",
            Path::group_term(Group::Seller, BtId(37)),
            "XRechnung: Seller city (BT-37) shall be present",
        ));
    }
}

fn br_de_4(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(
        inv.seller
            .address
            .as_ref()
            .and_then(|a| a.post_code.as_deref()),
    ) {
        report.push(Finding::fatal(
            "BR-DE-4",
            Path::group_term(Group::Seller, BtId(38)),
            "XRechnung: Seller post code (BT-38) shall be present",
        ));
    }
}

fn br_de_5(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.point.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-5",
            Path::group_term(Group::Seller, BtId(41)),
            "XRechnung: Seller contact point (BT-41) shall be present",
        ));
    }
}

fn br_de_6(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.phone.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-6",
            Path::group_term(Group::Seller, BtId(42)),
            "XRechnung: Seller contact telephone (BT-42) shall be present",
        ));
    }
}

fn br_de_7(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.email.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-7",
            Path::group_term(Group::Seller, BtId(43)),
            "XRechnung: Seller contact email (BT-43) shall be present",
        ));
    }
}

fn br_de_8(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(inv.buyer.address.as_ref().and_then(|a| a.city.as_deref())) {
        report.push(Finding::fatal(
            "BR-DE-8",
            Path::group_term(Group::Buyer, BtId(52)),
            "XRechnung: Buyer city (BT-52) shall be present",
        ));
    }
}

fn br_de_9(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(
        inv.buyer
            .address
            .as_ref()
            .and_then(|a| a.post_code.as_deref()),
    ) {
        report.push(Finding::fatal(
            "BR-DE-9",
            Path::group_term(Group::Buyer, BtId(53)),
            "XRechnung: Buyer post code (BT-53) shall be present",
        ));
    }
}

fn br_de_10(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(addr) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) else {
        return;
    };
    if blank(addr.city.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-10",
            Path::group_term(Group::Delivery, BtId(77)),
            "XRechnung: Deliver-to city (BT-77) shall be present when BG-15 is present",
        ));
    }
}

fn br_de_11(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(addr) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) else {
        return;
    };
    if blank(addr.post_code.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-11",
            Path::group_term(Group::Delivery, BtId(78)),
            "XRechnung: Deliver-to post code (BT-78) shall be present when BG-15 is present",
        ));
    }
}

fn br_de_14(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    for (i, row) in inv.tax_breakdown.iter().enumerate() {
        if row.rate.is_none() {
            report.push(Finding::fatal(
                "BR-DE-14",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "XRechnung: VAT category rate (BT-119) shall be present",
            ));
        }
    }
}

fn br_de_17(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    const ALLOWED: &[&str] = &["326", "380", "384", "389", "381", "875", "876", "877"];
    if !ALLOWED.contains(&code.as_str()) {
        report.push(Finding::warning(
            "BR-DE-17",
            Path::term(BtId(3)),
            format!(
                "XRechnung: invoice type code {} is not in the German supported set",
                code.as_str()
            ),
        ));
    }
}

fn means_code(inv: &Invoice) -> Option<&str> {
    inv.payment
        .as_ref()
        .and_then(|p| p.means_code.as_ref())
        .map(crate::code::Code::as_str)
}

fn br_de_23_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| c == "30" || c == "58")
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::CreditTransfer(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-23-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 30/58 requires credit transfer (BG-17)",
        ));
    }
}

fn br_de_24_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| matches!(c, "48" | "54" | "55"))
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::Card(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-24-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 48/54/55 requires payment card (BG-18)",
        ));
    }
}

fn br_de_25_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| c == "59")
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::DirectDebit(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-25-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 59 requires direct debit (BG-19)",
        ));
    }
}

fn br_de_23_b(_inv: &Invoice, _report: &mut Report) {
    // Unrepresentable: PaymentMeans is an enum.
}

fn br_de_30(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if let Some(PaymentMeans::DirectDebit(d)) = inv.payment.as_ref().and_then(|p| p.means.as_ref())
        && d.creditor_id
            .as_ref()
            .map(|c| c.value.trim().is_empty())
            .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-DE-30",
            Path::group_term(Group::Payment, BtId(90)),
            "XRechnung: BG-19 requires creditor identifier (BT-90)",
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

const fn rw(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Warning,
        text,
        source: Source::Crate,
        eval,
    }
}

/// Extra rules. Not CORE. Remaining KoSIT BR-DE-* / Extension / CVD stay in UNCOVERED.
pub static RULES: &[Rule] = &[
    r(
        "BR-DE-1",
        "XRechnung: Payment instructions (BG-16) shall be present.",
        br_de_1,
    ),
    r(
        "BR-DE-2",
        "XRechnung: Seller contact (BG-6) shall be present.",
        br_de_2,
    ),
    r(
        "BR-DE-3",
        "XRechnung: Seller city (BT-37) shall be present.",
        br_de_3,
    ),
    r(
        "BR-DE-4",
        "XRechnung: Seller post code (BT-38) shall be present.",
        br_de_4,
    ),
    r(
        "BR-DE-5",
        "XRechnung: Seller contact point (BT-41) shall be present when BG-6 exists.",
        br_de_5,
    ),
    r(
        "BR-DE-6",
        "XRechnung: Seller contact telephone (BT-42) shall be present when BG-6 exists.",
        br_de_6,
    ),
    r(
        "BR-DE-7",
        "XRechnung: Seller contact email (BT-43) shall be present when BG-6 exists.",
        br_de_7,
    ),
    r(
        "BR-DE-8",
        "XRechnung: Buyer city (BT-52) shall be present.",
        br_de_8,
    ),
    r(
        "BR-DE-9",
        "XRechnung: Buyer post code (BT-53) shall be present.",
        br_de_9,
    ),
    r(
        "BR-DE-10",
        "XRechnung: Deliver-to city (BT-77) when BG-15 is present.",
        br_de_10,
    ),
    r(
        "BR-DE-11",
        "XRechnung: Deliver-to post code (BT-78) when BG-15 is present.",
        br_de_11,
    ),
    r(
        "BR-DE-14",
        "XRechnung: VAT category rate (BT-119) shall be present on every BG-23 row.",
        br_de_14,
    ),
    r(
        "BR-DE-15",
        "XRechnung: Buyer reference (BT-10) shall be present.",
        br_de_15,
    ),
    r(
        "BR-DE-16",
        "XRechnung: seller VAT/tax id or tax representative when listed VAT categories are used.",
        br_de_16,
    ),
    rw(
        "BR-DE-17",
        "XRechnung: invoice type code should be one of 326/380/384/389/381/875/876/877 (warning).",
        br_de_17,
    ),
    r(
        "BR-DE-23-a",
        "XRechnung: BT-81 30/58 requires credit transfer (BG-17).",
        br_de_23_a,
    ),
    r(
        "BR-DE-24-a",
        "XRechnung: BT-81 48/54/55 requires payment card (BG-18).",
        br_de_24_a,
    ),
    r(
        "BR-DE-25-a",
        "XRechnung: BT-81 59 requires direct debit (BG-19).",
        br_de_25_a,
    ),
    r(
        "BR-DE-23-b",
        "XRechnung: BT-81 credit transfer forbids BG-18/BG-19 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    r(
        "BR-DE-24-b",
        "XRechnung: BT-81 card forbids BG-17/BG-19 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    r(
        "BR-DE-25-b",
        "XRechnung: BT-81 direct debit forbids BG-17/BG-18 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    r(
        "BR-DE-30",
        "XRechnung: BG-19 requires creditor identifier (BT-90).",
        br_de_30,
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

    #[test]
    fn br_de_1_is_payment_not_contact() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-1"),
            "{report}"
        );
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-2"),
            "{report}"
        );
        let mut inv = inv;
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: Some(Code::new("30")),
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-1"),
            "{report}"
        );
    }

    #[test]
    fn br_de_3_requires_seller_city() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-3"),
            "{report}"
        );
    }

    #[test]
    fn br_de_17_is_warning_not_fatal() {
        let mut inv = xr();
        inv.type_code = Some(Code::new("393"));
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-17")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);
    }
}
