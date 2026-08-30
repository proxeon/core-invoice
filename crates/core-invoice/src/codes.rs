//! Code lists. Hand-curated **subsets**; labelled incomplete vs full artefacts.
//!
//! CEN XML is EUPL — not in git. Generate from `spec/` after `task spec`.

use crate::bt::{BtId, Group, Path};
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

/// ISO 4217 alphabetic codes we accept. Subset; `XXX` is in ISO 4217.
/// UNCOVERED: remaining ISO 4217 entries vs full table.
const CURRENCIES: &[&str] = &[
    "AED", "AUD", "BDT", "BHD", "BND", "BRL", "CAD", "CHF", "CNY", "CZK", "DKK", "EGP", "EUR",
    "GBP", "HKD", "HUF", "IDR", "INR", "JPY", "KES", "KRW", "KWD", "LKR", "MXN", "MYR", "NOK",
    "NZD", "OMR", "PHP", "PKR", "PLN", "QAR", "RON", "RUB", "SAR", "SEK", "SGD", "THB", "TRY",
    "TWD", "USD", "VND", "ZAR", "XXX",
];

/// ISO 3166-1 alpha-2 subset. UNCOVERED vs full table.
const COUNTRIES: &[&str] = &[
    "AT", "AU", "BE", "BG", "BN", "BR", "CA", "CH", "CN", "CY", "CZ", "DE", "DK", "EE", "EG", "ES",
    "FI", "FR", "GB", "GR", "HK", "HR", "HU", "ID", "IE", "IN", "IT", "JP", "KR", "LT", "LU", "LV",
    "MT", "MX", "MY", "NL", "NO", "NZ", "PH", "PL", "PT", "RO", "SA", "SE", "SG", "SI", "SK", "TH",
    "TR", "US", "VN", "ZA",
];

/// UNTDID 1001 invoice-related subset (not credit-note).
const INVOICE_TYPE_CODES: &[&str] = &[
    "80", "82", "84", "130", "202", "218", "219", "325", "326", "331", "380", "382", "383", "384",
    "385", "386", "387", "388", "389", "390", "393", "394", "395", "456", "457", "458", "527",
    "870", "875", "876", "877", "935",
];

/// UNTDID 1001 credit-note-related subset. Overlaps invoice only at `81`-family
/// codes such as `81` itself — `381` is credit-note only.
const CREDIT_NOTE_TYPE_CODES: &[&str] = &[
    "81", "83", "261", "262", "296", "308", "381", "396", "420", "458", "527", "532",
];

/// UNCL 5305 VAT categories. Not PINT-MY TaxCat.
const UNCL_5305: &[&str] = &["S", "Z", "E", "AE", "K", "G", "O", "L", "M", "B"];

/// UNCL 4461 payment means subset. MY Z0x are profile extras, not this list.
const UNCL_4461: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "30", "31", "42", "48", "49", "57", "58",
    "59", "68",
];

const UNITS: &[&str] = &[
    "C62", "H87", "KGM", "LTR", "MTR", "MTK", "MTQ", "HUR", "DAY", "MON", "TNE",
];
const MIME: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "text/csv",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.oasis.opendocument.spreadsheet",
];
const EAS: &[&str] = &[
    "0002", "0088", "0096", "0190", "0192", "0204", "0230", "9944", "9952",
];
const VATEX: &[&str] = &[
    "VATEX-EU-AE",
    "VATEX-EU-D",
    "VATEX-EU-F",
    "VATEX-EU-G",
    "VATEX-EU-I",
    "VATEX-EU-IC",
    "VATEX-EU-O",
    "VATEX-EU-J",
];

pub fn currency(code: &str) -> bool {
    CURRENCIES.iter().any(|c| c.eq_ignore_ascii_case(code))
}
pub fn country(code: &str) -> bool {
    COUNTRIES.iter().any(|c| c.eq_ignore_ascii_case(code))
}
pub fn uncl_5305(code: &str) -> bool {
    UNCL_5305.iter().any(|c| c.eq_ignore_ascii_case(code))
}
pub fn invoice_type(code: &str) -> bool {
    INVOICE_TYPE_CODES.contains(&code)
}
pub fn credit_note_type(code: &str) -> bool {
    CREDIT_NOTE_TYPE_CODES.contains(&code)
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
            format!("BT-6 {} is not an ISO 4217 alphabetic code", code),
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
    if !UNCL_4461.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "BR-CL-16",
            Path::group_term(Group::Payment, BtId(81)),
            format!(
                "BT-81 {} is not in UNCL 4461 (MY Z0x are profile extras)",
                code
            ),
        ));
    }
}

fn vat_profile(inv: &Invoice) -> bool {
    matches!(inv.profile, Profile::En16931 | Profile::PeppolBis3)
}

fn br_cl_17(inv: &Invoice, report: &mut Report) {
    if !vat_profile(inv) {
        return;
    }
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
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
        if line.tax.system != TaxSystem::Vat {
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
        if !VATEX.iter().any(|c| c.eq_ignore_ascii_case(code.as_str())) {
            report.push(Finding::fatal(
                "BR-CL-22",
                Path::at_term(Group::TaxBreakdown, i, BtId(121)),
                format!("BT-121 {} is not a VATEX code", code),
            ));
        }
    }
}

fn br_cl_23(inv: &Invoice, report: &mut Report) {
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(u) = line.unit.as_ref() else {
            continue;
        };
        if !UNITS.contains(&u.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-23",
                Path::at_term(Group::Line, i, BtId(130)),
                format!("BT-130 {u} is not Rec 20/21 in this subset"),
            ));
        }
    }
}

fn br_cl_24(inv: &Invoice, report: &mut Report) {
    for (i, doc) in inv.supporting_documents.iter().enumerate() {
        let Some(att) = doc.attachment.as_ref() else {
            continue;
        };
        if !MIME.contains(&att.mime.as_str()) {
            report.push(Finding::fatal(
                "BR-CL-24",
                Path::at_term(Group::Attachment, i, BtId(125)),
                format!("mime {} is not in the subset", att.mime),
            ));
        }
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
        if !EAS.contains(&scheme) {
            report.push(Finding::fatal(
                "BR-CL-25",
                Path::group_term(group, BtId(bt)),
                format!("EAS {scheme} is not in the subset"),
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
        "BR-CL-04",
        "Invoice currency code MUST be coded using ISO 4217 alpha-3.",
        br_cl_04,
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
        "BR-CL-16",
        "Payment means code MUST be coded using UNCL 4461.",
        br_cl_16,
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
        "Unit codes MUST be coded using UNECE Rec 20 / Rec 21 (subset).",
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
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::{Invoice, Party};
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
}
