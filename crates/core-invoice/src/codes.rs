//! Code lists generated from `refers/` genericode (code points only; EUPL XML stays out of git).
//!
//! `task lists` / `python3 xtask/gen_codes.py` refreshes [`generated_codes`].

use crate::bt::{BtId, Group, Path};
use crate::generated_codes as lists;
use crate::invoice::Invoice;
use crate::kind::DocumentKind;
use crate::profile::Profile;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;
use crate::tax::TaxSystem;

/// CEN EN 16931 validation artefacts pin. Fully-qualified tag, not the branch.
pub const ARTEFACT_VERSION: &str = "validation-1.3.16";
pub const PEPPOL_BIS_VERSION: &str = "v3.0.20";
pub const PINT_MY_VERSION: &str = "1.3.0";
/// ConnectingEurope/eInvoicing-EN16931 release tag commit (docs/spec.md).
pub const EN16931_GIT: &str = "b6c9e06";
pub const PINT_VERSION: &str = "1.1.2";

fn listed(list: &[&str], code: &str) -> bool {
    list.iter().any(|c| c.eq_ignore_ascii_case(code))
}

pub fn currency(code: &str) -> bool {
    listed(lists::ISO_4217, code)
}
pub fn country(code: &str) -> bool {
    listed(lists::ISO_3166, code)
}
pub fn uncl_5305(code: &str) -> bool {
    listed(lists::UNCL_5305, code)
}
pub fn invoice_type(code: &str) -> bool {
    lists::UNCL_1001_INVOICE.contains(&code)
}
pub fn credit_note_type(code: &str) -> bool {
    lists::UNCL_1001_CREDIT_NOTE.contains(&code)
}
pub fn eas(code: &str) -> bool {
    lists::EAS.contains(&code)
}
pub fn vatex(code: &str) -> bool {
    listed(lists::VATEX, code)
}
pub fn unit(code: &str) -> bool {
    lists::REC20.contains(&code)
}
pub fn mime(code: &str) -> bool {
    lists::MIME.contains(&code)
}
pub fn icd(code: &str) -> bool {
    lists::ICD.contains(&code)
}
pub fn uncl_1153(code: &str) -> bool {
    listed(lists::UNCL_1153, code)
}
pub fn uncl_4451(code: &str) -> bool {
    listed(lists::UNCL_4451, code)
}
pub fn pint_my_taxcat(code: &str) -> bool {
    listed(lists::PINT_MY_TAXCAT, code)
}

pub mod guard {
    use crate::profile::Profile;

    /// EAS membership with a withdrawn-successor hint. No network.
    pub fn eas(code: &str, profile: Profile) -> Result<(), String> {
        match code {
            "9958" => Err("EAS 9958 is withdrawn; use 0204".into()),
            "T" if profile == Profile::PintMy => {
                Err("PINT-MY tax category T is withdrawn; use SA/SE/HVG/LVG".into())
            }
            _ => Ok(()),
        }
    }
}

fn br_cl_01(inv: &Invoice, report: &mut Report) {
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    let ok = match inv.kind {
        DocumentKind::Invoice => invoice_type(code.as_str()),
        DocumentKind::CreditNote => credit_note_type(code.as_str()),
    };
    if !ok {
        report.push(Finding::fatal(
            "BR-CL-01",
            Path::term(BtId(3)),
            format!(
                "type code {} is not in the UNTDID 1001 list for {:?}",
                code, inv.kind
            ),
        ));
    }
}

fn br_cl_03(_inv: &Invoice, _report: &mut Report) {
    // BR-CL-03: @currencyID ∈ ISO 4217. Wire-only; formats::validate_xml walks attributes.
}

fn br_cl_08(inv: &Invoice, report: &mut Report) {
    // BR-CL-08: BT-21 note subject, restriction of UNTDID 4451. Absent subject does not fire.
    for (i, n) in inv.notes.iter().enumerate() {
        let Some(code) = n.subject.as_ref() else {
            continue;
        };
        if !uncl_4451(code.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-08",
                Path::at_term(Group::Document, i, BtId(21)),
                format!("note subject {code} is not in UNTDID 4451 (EN restriction)"),
            ));
        }
    }
}

fn br_cl_04(inv: &Invoice, report: &mut Report) {
    if inv.currency.trim().is_empty() {
        return;
    }
    if !currency(&inv.currency) {
        report.push(Finding::fatal(
            "BR-CL-04",
            Path::term(BtId(5)),
            format!("BT-5 {} is not an ISO 4217 alphabetic code", inv.currency),
        ));
    }
}

fn br_cl_05(inv: &Invoice, report: &mut Report) {
    let Some(code) = inv.tax_currency.as_ref() else {
        return;
    };
    if !currency(code.as_str()) {
        report.push(Finding::fatal(
            "BR-CL-05",
            Path::term(BtId(6)),
            format!("BT-6 {code} is not an ISO 4217 alphabetic code"),
        ));
    }
}

fn br_cl_14(inv: &Invoice, report: &mut Report) {
    for (party, group, bt) in [
        (&inv.seller, Group::Seller, 40u16),
        (&inv.buyer, Group::Buyer, 55u16),
    ] {
        if party.country().trim().is_empty() {
            continue;
        }
        if !country(party.country()) {
            report.push(Finding::fatal(
                "BR-CL-14",
                Path::group_term(group, BtId(bt)),
                format!("country {} is not ISO 3166-1 alpha-2", party.country()),
            ));
        }
    }
}

fn br_cl_16(inv: &Invoice, report: &mut Report) {
    let Some(pay) = inv.payment.as_ref() else {
        return;
    };
    let Some(code) = pay.means_code.as_ref() else {
        return;
    };
    let ok = lists::UNCL_4461.contains(&code.as_str())
        || (inv.profile == Profile::PintMy && pint_my_payment(code.as_str()));
    if !ok {
        report.push(Finding::fatal(
            "BR-CL-16",
            Path::group_term(Group::Payment, BtId(81)),
            format!("BT-81 {code} is not in UNCL 4461 (MY Z0x are profile extras)"),
        ));
    }
}

/// Z01/Z03–Z08 are PINT-MY extras on BT-81, not UNCL 4461 membership for EN/Peppol.
fn pint_my_payment(code: &str) -> bool {
    matches!(code, "Z01" | "Z03" | "Z04" | "Z05" | "Z06" | "Z07" | "Z08")
}

fn vat_profile(inv: &Invoice) -> bool {
    matches!(inv.profile, Profile::En16931 | Profile::PeppolBis3)
}

fn br_cl_17(inv: &Invoice, report: &mut Report) {
    if !vat_profile(inv) {
        return;
    }
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
        if e.category.as_str().trim().is_empty() {
            continue;
        }
        if !uncl_5305(e.category.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-17",
                Path::at_term(Group::TaxBreakdown, i, BtId(118)),
                format!("BT-118 {} is not UNCL 5305", e.category),
            ));
        }
    }
}

fn br_cl_18(inv: &Invoice, report: &mut Report) {
    if !vat_profile(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        if line.tax.system != TaxSystem::Vat || line.tax.code.trim().is_empty() {
            continue;
        }
        if !uncl_5305(&line.tax.code) {
            report.push(Finding::fatal(
                "BR-CL-18",
                Path::at_term(Group::Line, i, BtId(151)),
                format!("BT-151 {} is not UNCL 5305", line.tax.code),
            ));
        }
    }
}

fn br_cl_22(inv: &Invoice, report: &mut Report) {
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
        let Some(code) = e.exemption_code.as_ref() else {
            continue;
        };
        if !vatex(code.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-22",
                Path::at_term(Group::TaxBreakdown, i, BtId(121)),
                format!("BT-121 {code} is not a VATEX code"),
            ));
        }
    }
}

fn br_cl_23(inv: &Invoice, report: &mut Report) {
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(u) = line.unit.as_ref() else {
            continue;
        };
        if !unit(u.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-23",
                Path::at_term(Group::Line, i, BtId(130)),
                format!("BT-130 {u} is not UNECE Rec 20/21"),
            ));
        }
    }
}

fn br_cl_24(inv: &Invoice, report: &mut Report) {
    for (i, doc) in inv.supporting_documents.iter().enumerate() {
        let Some(att) = doc.attachment.as_ref() else {
            continue;
        };
        if att.mime.trim().is_empty() {
            continue;
        }
        if !mime(att.mime.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-24",
                Path::at_term(Group::Attachment, i, BtId(125)),
                format!("mime {} is not in the subset", att.mime),
            ));
        }
    }
}

fn br_cl_07(inv: &Invoice, report: &mut Report) {
    // BR-CL-07: BT-18 / BT-128 scheme (when present) is UNTDID 1153, not ICD.
    if let Some(scheme) = inv
        .invoiced_object
        .as_ref()
        .and_then(|id| id.scheme.as_deref())
        && !uncl_1153(scheme)
    {
        report.push(Finding::fatal(
            "BR-CL-07",
            Path::term(BtId(18)),
            format!("object identifier scheme {scheme} is not UNTDID 1153"),
        ));
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(scheme) = line
            .invoiced_object
            .as_ref()
            .and_then(|id| id.scheme.as_deref())
        else {
            continue;
        };
        if !uncl_1153(scheme) {
            report.push(Finding::fatal(
                "BR-CL-07",
                Path::at_term(Group::Line, i, BtId(128)),
                format!("object identifier scheme {scheme} is not UNTDID 1153"),
            ));
        }
    }
}

fn br_cl_10(inv: &Invoice, report: &mut Report) {
    // BR-CL-10: PartyIdentification scheme is ISO 6523 ICD. SEPA allowed on seller/payee.
    let parties = [
        (&inv.seller.identifiers[..], Group::Seller, 29u16, true),
        (&inv.buyer.identifiers[..], Group::Buyer, 46u16, false),
    ];
    for (ids, group, bt, sepa_ok) in parties {
        for id in ids {
            let Some(scheme) = id.scheme.as_deref() else {
                continue;
            };
            let ok = icd(scheme) || (sepa_ok && scheme.eq_ignore_ascii_case("SEPA"));
            if !ok {
                report.push(Finding::fatal(
                    "BR-CL-10",
                    Path::group_term(group, BtId(bt)),
                    format!("identifier scheme {scheme} is not ISO 6523 ICD"),
                ));
            }
        }
    }
    if let Some(payee) = inv.payee.as_ref()
        && let Some(id) = payee.identifier.as_ref()
        && let Some(scheme) = id.scheme.as_deref()
        && !(icd(scheme) || scheme.eq_ignore_ascii_case("SEPA"))
    {
        report.push(Finding::fatal(
            "BR-CL-10",
            Path::term(BtId(60)),
            format!("payee identifier scheme {scheme} is not ISO 6523 ICD"),
        ));
    }
}

fn br_cl_11(inv: &Invoice, report: &mut Report) {
    // BR-CL-11: CompanyID scheme is ICD when present. Unschemed (PINT-MY BRN) does not fire.
    for (reg, group, bt) in [
        (inv.seller.legal_registration.as_ref(), Group::Seller, 30u16),
        (inv.buyer.legal_registration.as_ref(), Group::Buyer, 47u16),
        (
            inv.payee
                .as_ref()
                .and_then(|p| p.legal_registration.as_ref()),
            Group::Seller,
            61u16,
        ),
    ] {
        let Some(id) = reg else {
            continue;
        };
        let Some(scheme) = id.scheme.as_deref() else {
            continue;
        };
        if !icd(scheme) {
            report.push(Finding::fatal(
                "BR-CL-11",
                Path::group_term(group, BtId(bt)),
                format!("legal registration scheme {scheme} is not ISO 6523 ICD"),
            ));
        }
    }
}

fn br_cl_21(inv: &Invoice, report: &mut Report) {
    // BR-CL-21: BT-157 StandardItemIdentification scheme is ICD. Not BT-155 item_id.
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(scheme) = line
            .standard_id
            .as_ref()
            .and_then(|id| id.scheme.as_deref())
        else {
            continue;
        };
        if !icd(scheme) {
            report.push(Finding::fatal(
                "BR-CL-21",
                Path::at_term(Group::Line, i, BtId(157)),
                format!("BT-157 scheme {scheme} is not ISO 6523 ICD"),
            ));
        }
    }
}

fn br_cl_26(inv: &Invoice, report: &mut Report) {
    // BR-CL-26: DeliveryLocation/ID scheme is ICD.
    let Some(scheme) = inv
        .delivery
        .as_ref()
        .and_then(|d| d.location_id.as_ref())
        .and_then(|id| id.scheme.as_deref())
    else {
        return;
    };
    if !icd(scheme) {
        report.push(Finding::fatal(
            "BR-CL-26",
            Path::term(BtId(71)),
            format!("deliver-to location scheme {scheme} is not ISO 6523 ICD"),
        ));
    }
}

fn br_cl_25(inv: &Invoice, report: &mut Report) {
    for (party, group, bt) in [
        (&inv.seller, Group::Seller, 34u16),
        (&inv.buyer, Group::Buyer, 49u16),
    ] {
        let Some(ep) = party.electronic_address.as_ref() else {
            continue;
        };
        let Some(scheme) = ep.scheme.as_deref() else {
            continue;
        };
        if !eas(scheme) {
            report.push(Finding::fatal(
                "BR-CL-25",
                Path::group_term(group, BtId(bt)),
                format!("EAS {scheme} is not in the Electronic Address Identifier Scheme list"),
            ));
        }
    }
}

fn br_cl_06(inv: &Invoice, report: &mut Report) {
    let Some(code) = inv.tax_point_code.as_ref() else {
        return;
    };
    // BR-CL-06: BT-8 is UNCL 2005 subset 3 / 35 / 432.
    if !lists::UNCL_2005.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "BR-CL-06",
            Path::term(BtId(8)),
            format!("BT-8 {code} is not UNCL 2005 (3, 35, 432)"),
        ));
    }
}

fn br_cl_13(inv: &Invoice, report: &mut Report) {
    for (i, line) in inv.lines.iter().enumerate() {
        for cl in &line.classifications {
            let Some(scheme) = cl.scheme.as_deref() else {
                continue;
            };
            // BR-CL-13 / IBR-CL-13: Item classification listID is UNCL 7143 (CG is CLASS in PINT-MY).
            if !lists::UNCL_7143.contains(&scheme) {
                report.push(Finding::fatal(
                    "BR-CL-13",
                    Path::at_term(Group::Line, i, BtId(158)),
                    format!("classification listID {scheme} is not UNCL 7143"),
                ));
            }
        }
    }
}

fn br_cl_15(inv: &Invoice, report: &mut Report) {
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(c) = line.origin_country.as_ref() else {
            continue;
        };
        if !country(c.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-15",
                Path::at_term(Group::Line, i, BtId(159)),
                format!("BT-159 {c} is not ISO 3166-1 alpha-2"),
            ));
        }
    }
}

fn br_cl_19(inv: &Invoice, report: &mut Report) {
    for (i, a) in inv.document_allowances.iter().enumerate() {
        let Some(code) = a.reason_code.as_ref() else {
            continue;
        };
        if !lists::UNCL_5189.contains(&code.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-19",
                Path::at_term(Group::DocumentAllowance, i, BtId(98)),
                format!("BT-98 {code} is not UNCL 5189"),
            ));
        }
    }
}

fn br_cl_20(inv: &Invoice, report: &mut Report) {
    for (i, a) in inv.document_charges.iter().enumerate() {
        let Some(code) = a.reason_code.as_ref() else {
            continue;
        };
        if !lists::UNCL_7161.contains(&code.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-20",
                Path::at_term(Group::DocumentCharge, i, BtId(105)),
                format!("BT-105 {code} is not UNCL 7161"),
            ));
        }
    }
}

const fn r(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Fatal,
        text,
        source: Source::ArtefactOnly,
        eval,
    }
}

pub static RULES: &[Rule] = &[
    r(
        "BR-CL-01",
        "Document type code MUST be coded by the invoice and credit note related code lists of UNTDID 1001.",
        br_cl_01,
    ),
    r(
        "BR-CL-03",
        "currencyID MUST be coded using ISO 4217 alpha-3 (wire @currencyID).",
        br_cl_03,
    ),
    r(
        "BR-CL-04",
        "Invoice currency code MUST be coded using ISO 4217 alpha-3.",
        br_cl_04,
    ),
    r(
        "BR-CL-08",
        "Invoice note subject code (BT-21) MUST be coded using UNCL 4451.",
        br_cl_08,
    ),
    r(
        "BR-CL-05",
        "Tax accounting currency MUST be coded using ISO 4217 alpha-3.",
        br_cl_05,
    ),
    r(
        "BR-CL-14",
        "Country codes MUST be coded using ISO 3166-1 alpha-2.",
        br_cl_14,
    ),
    r(
        "BR-CL-06",
        "VAT point date code (BT-8) MUST be coded using UNCL 2005 (3, 35, 432).",
        br_cl_06,
    ),
    r(
        "BR-CL-13",
        "Item classification scheme (BT-158-1) MUST be coded using UNCL 7143.",
        br_cl_13,
    ),
    r(
        "BR-CL-15",
        "Item origin country (BT-159) MUST be coded using ISO 3166-1 alpha-2.",
        br_cl_15,
    ),
    r(
        "BR-CL-16",
        "Payment means code MUST be coded using UNCL 4461.",
        br_cl_16,
    ),
    r(
        "BR-CL-19",
        "Document allowance reason code MUST be coded using UNCL 5189.",
        br_cl_19,
    ),
    r(
        "BR-CL-20",
        "Document charge reason code MUST be coded using UNCL 7161.",
        br_cl_20,
    ),
    r(
        "BR-CL-17",
        "VAT category code (BT-118) MUST be coded using UNCL 5305 (VAT profiles only).",
        br_cl_17,
    ),
    r(
        "BR-CL-18",
        "Invoiced item VAT category code (BT-151) MUST be coded using UNCL 5305 (VAT profiles only).",
        br_cl_18,
    ),
    r(
        "BR-CL-22",
        "VAT exemption reason code MUST be coded using the VATEX list (case-insensitive).",
        br_cl_22,
    ),
    r(
        "BR-CL-23",
        "Unit codes MUST be coded using UNECE Rec 20 (generated list). Rec 21 is not generated.",
        br_cl_23,
    ),
    r(
        "BR-CL-24",
        "Attachment mime code MUST be from the allowed MIME list (subset).",
        br_cl_24,
    ),
    r(
        "BR-CL-25",
        "Electronic address scheme MUST be from EAS (subset).",
        br_cl_25,
    ),
    r(
        "BR-CL-07",
        "Object identifier identification scheme (BT-18 / BT-128) MUST be coded using UNTDID 1153.",
        br_cl_07,
    ),
    r(
        "BR-CL-10",
        "Party identifier scheme MUST be ISO 6523 ICD (SEPA allowed on seller/payee).",
        br_cl_10,
    ),
    r(
        "BR-CL-11",
        "Legal registration identifier scheme MUST be ISO 6523 ICD when present.",
        br_cl_11,
    ),
    r(
        "BR-CL-21",
        "Item standard identifier scheme (BT-157) MUST be ISO 6523 ICD.",
        br_cl_21,
    ),
    r(
        "BR-CL-26",
        "Deliver-to location identifier scheme MUST be ISO 6523 ICD.",
        br_cl_26,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::{Invoice, Party};
    use crate::rules::explain;
    use crate::validate;

    #[test]
    fn us_dollar_sign_fails_cl04_eur_passes() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "US$",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(crate::code::Code::new("380"));
        inv.lines = vec![crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        )];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-04"),
            "{report}"
        );
        inv.currency = "EUR".into();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CL-04"),
            "{report}"
        );
        inv.currency = "XXX".into();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CL-04"),
            "{report}"
        );
    }

    #[test]
    fn br_cl_08_note_subject_4451() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.notes.push(crate::invoice::InvoiceNote {
            subject: Some(crate::code::Code::new("NOPE")),
            text: "x".into(),
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-08"),
            "{report}"
        );
        inv.notes[0].subject = Some(crate::code::Code::new("AAA"));
        assert!(validate(&inv).findings.iter().all(|f| f.id != "BR-CL-08"));
        inv.notes[0].subject = None;
        assert!(validate(&inv).findings.iter().all(|f| f.id != "BR-CL-08"));
        assert!(explain("BR-CL-08").unwrap().contains("4451"));
        assert!(explain("BR-CL-03").unwrap().contains("currencyID"));
    }

    #[test]
    fn invoice_381_fails_cl01() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(crate::code::Code::new("381"));
        inv.kind = DocumentKind::Invoice;
        inv.lines = vec![crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        )];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-01"),
            "{report}"
        );
    }

    #[test]
    fn artefact_pins_are_fully_qualified() {
        assert_eq!(ARTEFACT_VERSION, "validation-1.3.16");
        assert_eq!(PEPPOL_BIS_VERSION, "v3.0.20");
        assert_eq!(PINT_MY_VERSION, "1.3.0");
    }

    #[test]
    fn br_cl_07_rejects_non_1153_scheme() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.invoiced_object = Some(crate::identifier::Identifier::schemed("X", "NOPE"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-07"),
            "{report}"
        );
    }

    #[test]
    fn br_cl_21_binds_standard_id_not_item_id() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        let mut line = crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        );
        line.item_id = Some(crate::identifier::Identifier::schemed("SKU", "FOO"));
        line.standard_id = Some(crate::identifier::Identifier::schemed("GTIN", "FOO"));
        inv.lines = vec![line];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-21"),
            "{report}"
        );
    }

    #[test]
    fn invoice_326_is_not_br_cl_01() {
        let mut inv = Invoice::blank(
            Profile::PeppolBis3,
            "1",
            "EUR",
            {
                let mut p = Party::new("S", "DE");
                p.electronic_address = Some(crate::identifier::Identifier::schemed("1", "0088"));
                p
            },
            {
                let mut p = Party::new("B", "DE");
                p.electronic_address = Some(crate::identifier::Identifier::schemed("2", "0088"));
                p
            },
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(crate::code::Code::new("326"));
        inv.specification_id = Some(Profile::PEPPOL_BIS3_PREFIX.into());
        inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CL-01"),
            "{report}"
        );
    }
}
