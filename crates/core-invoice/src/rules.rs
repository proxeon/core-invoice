use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::numeric::Percentage;
use crate::report::{Finding, Report, Severity, Source};

#[derive(Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub severity: Severity,
    pub text: &'static str,
    pub source: Source,
    pub eval: fn(&Invoice, &mut Report),
}

pub fn matches_id(registered: &str, query: &str) -> bool {
    let a = canonical(registered);
    let b = canonical(query);
    a.eq_ignore_ascii_case(&b)
}

fn canonical(id: &str) -> String {
    let id = id.trim();
    let Some((head, tail)) = id.rsplit_once('-') else {
        return id.to_ascii_uppercase();
    };
    if tail.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}-{tail:0>2}", head.to_ascii_uppercase());
    }
    id.to_ascii_uppercase()
}

pub fn explain(id: &str) -> Option<&'static str> {
    catalogue()
        .iter()
        .find(|r| matches_id(r.id, id))
        .map(|r| r.text)
}

pub fn catalogue() -> &'static [Rule] {
    static CELL: std::sync::OnceLock<Vec<Rule>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        ALL.iter()
            .copied()
            .chain(crate::category::RULES.iter().copied())
            .chain(crate::codes::RULES.iter().copied())
            .collect()
    })
}

fn spec_lookup(invoice: &Invoice, report: &mut Report) {
    let Some(id) = invoice.specification_id.as_deref() else {
        return;
    };
    if id.contains('*') {
        report.push(Finding::fatal(
            "IBR-SR-63",
            Path::term(BtId(24)),
            "BT-24 shall not contain '*' (wildcard is an SMP capability, not an instance id)",
        ));
        return;
    }
    match crate::profile::Profile::for_specification_id(id) {
        crate::profile::ProfileLookup::WrongProcess => {
            report.push(Finding::fatal(
                "CORE-PROCESS-01",
                Path::term(BtId(24)),
                "Specification identifier is a self-billing (or other) process; not billing",
            ));
        }
        crate::profile::ProfileLookup::Unknown => {
            report.push(Finding::fatal(
                "CORE-SPEC-01",
                Path::term(BtId(24)),
                "Unrecognised specification identifier (BT-24)",
            ));
        }
        crate::profile::ProfileLookup::Profile(_) => {}
    }
}

fn br_01(invoice: &Invoice, report: &mut Report) {
    if invoice
        .specification_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        report.push(Finding::fatal(
            "BR-01",
            Path::term(BtId(24)),
            "An Invoice shall have a Specification identifier (BT-24)",
        ));
    }
}

fn br_03(invoice: &Invoice, report: &mut Report) {
    if invoice.issue_date.is_none() {
        report.push(Finding::fatal(
            "BR-03",
            Path::term(BtId(2)),
            "An Invoice shall have an Invoice issue date (BT-2)",
        ));
    }
}

fn br_04(invoice: &Invoice, report: &mut Report) {
    if invoice
        .type_code
        .as_ref()
        .map(|c| c.is_empty())
        .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-04",
            Path::term(BtId(3)),
            "An Invoice shall have an Invoice type code (BT-3)",
        ));
    }
}

fn br_09(invoice: &Invoice, report: &mut Report) {
    if invoice.seller.country.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-09",
            Path::term(BtId(40)),
            "The Seller postal address shall contain a Seller country code (BT-40)",
        ));
    }
}

fn br_11(invoice: &Invoice, report: &mut Report) {
    if invoice.buyer.country.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-11",
            Path::term(BtId(55)),
            "The Buyer postal address shall contain a Buyer country code (BT-55)",
        ));
    }
}

fn br_21(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.id.trim().is_empty() {
            report.push(Finding::fatal(
                "BR-21",
                Path::at_term(Group::Line, i, BtId(126)),
                "Each Invoice line shall have an Invoice line identifier (BT-126)",
            ));
        }
    }
}

fn br_25(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.name.trim().is_empty() {
            report.push(Finding::fatal(
                "BR-25",
                Path::at_term(Group::Line, i, BtId(153)),
                "Each Invoice line shall have an Item name (BT-153)",
            ));
        }
    }
}

fn br_02(invoice: &Invoice, report: &mut Report) {
    if invoice.number.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-02",
            Path::term(BtId(1)),
            "Invoice number (BT-1) shall be present",
        ));
    }
}

fn br_05(invoice: &Invoice, report: &mut Report) {
    if invoice.currency.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-05",
            Path::term(BtId(5)),
            "Invoice currency code (BT-5) shall be present",
        ));
    }
}

fn br_06(invoice: &Invoice, report: &mut Report) {
    if invoice.seller.name.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-06",
            Path::term(BtId(27)),
            "Seller name (BT-27) shall be present",
        ));
    }
}

fn br_07(invoice: &Invoice, report: &mut Report) {
    if invoice.buyer.name.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-07",
            Path::term(BtId(44)),
            "Buyer name (BT-44) shall be present",
        ));
    }
}

fn br_16(invoice: &Invoice, report: &mut Report) {
    if invoice.lines.is_empty() {
        report.push(Finding::fatal(
            "BR-16",
            Path::group(Group::Line),
            "An invoice shall have at least one Invoice line (BG-25)",
        ));
    }
}

fn pint_tax(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if !invoice.profile.allows(line.tax.system) {
            report.push(Finding::fatal(
                "PINT-TAX",
                Path::at_term(Group::Line, i, BtId(151)),
                format!(
                    "Tax system {} is not allowed on profile {}",
                    line.tax.system.as_str(),
                    invoice.profile.slug()
                ),
            ));
        }
    }
}

fn ibr_my(invoice: &Invoice, report: &mut Report) {
    if invoice.profile != crate::profile::Profile::PintMy {
        return;
    }
    if invoice.seller.legal_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-02-MY",
            Path::term(BtId(30)),
            "Seller legal registration identifier (BRN) shall be present",
        ));
    }
    if invoice.buyer.legal_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-03-MY",
            Path::term(BtId(47)),
            "Buyer legal registration identifier (BRN) shall be present",
        ));
    }
    if invoice.seller.tax_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-04-MY",
            Path::term(BtId(32)),
            "Seller TIN (tax registration) shall be present",
        ));
    }
    for (i, line) in invoice.lines.iter().enumerate() {
        if !crate::tax::pint_my_category(&line.tax.code) {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-CL-01-MY",
                Path::at_term(Group::Line, i, BtId(151)),
                format!(
                    "Tax category {} is not a PINT-MY code (SA SE HVG LVG TTX E O)",
                    line.tax.code
                ),
            ));
        }
    }
}

fn totals_of(invoice: &Invoice) -> Option<&crate::invoice::DocumentTotals> {
    invoice.totals.as_ref()
}

fn overflow(report: &mut Report, id: &'static str, term: u16, label: &str) {
    report.push(Finding::fatal(
        id,
        Path::group_term(Group::Totals, BtId(term)),
        format!("{label} overflowed; amounts are not representable"),
    ));
}

fn br_co_10(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(expected) =
        crate::amount::InvoiceAmount::checked_sum(invoice.lines.iter().map(|l| l.net))
    else {
        overflow(report, "BR-CO-10", 106, "BT-106");
        return;
    };
    match totals.line_net {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-10",
            Path::group_term(Group::Totals, BtId(106)),
            format!("BT-106 {stated} ≠ Σ BT-131 {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-10",
            Path::group_term(Group::Totals, BtId(106)),
            format!("BT-106 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_11(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(107));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(
        invoice.document_allowances.iter().map(|a| a.amount),
    ) else {
        overflow(report, "BR-CO-11", 107, "BT-107");
        return;
    };
    match (
        invoice.document_allowances.is_empty(),
        totals.allowance_total,
    ) {
        (true, None) => {}
        (true, Some(stated)) => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 shall be absent when there is no BG-20 (found {stated})"),
        )),
        (false, None) => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 is absent; expected Σ BT-92 {expected}"),
        )),
        (false, Some(stated)) if stated != expected => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 {stated} ≠ Σ BT-92 {expected}"),
        )),
        _ => {}
    }
}

fn br_co_12(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(108));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(
        invoice.document_charges.iter().map(|c| c.amount),
    ) else {
        overflow(report, "BR-CO-12", 108, "BT-108");
        return;
    };
    match (invoice.document_charges.is_empty(), totals.charge_total) {
        (true, None) => {}
        (true, Some(stated)) => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 shall be absent when there is no BG-21 (found {stated})"),
        )),
        (false, None) => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 is absent; expected Σ BT-99 {expected}"),
        )),
        (false, Some(stated)) if stated != expected => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 {stated} ≠ Σ BT-99 {expected}"),
        )),
        _ => {}
    }
}

fn br_co_13(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(line_net) = totals.line_net else {
        return;
    };
    let expected = match (totals.allowance_total, totals.charge_total) {
        (None, None) => Some(line_net),
        (Some(a), None) => line_net.checked_sub(a),
        (None, Some(c)) => line_net.checked_add(c),
        (Some(a), Some(c)) => line_net.checked_sub(a).and_then(|v| v.checked_add(c)),
    };
    let Some(expected) = expected else {
        overflow(report, "BR-CO-13", 109, "BT-109");
        return;
    };
    match totals.without_tax {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-13",
            Path::group_term(Group::Totals, BtId(109)),
            format!("BT-109 {stated} ≠ BT-106 − BT-107 + BT-108 = {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-13",
            Path::group_term(Group::Totals, BtId(109)),
            format!("BT-109 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_14(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(110));
    let rows = invoice
        .tax_breakdown
        .iter()
        .filter(|e| crate::reconcile::counts_toward_tax_total(invoice.profile, e));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(rows.map(|e| e.tax)) else {
        overflow(report, "BR-CO-14", 110, "BT-110");
        return;
    };
    match totals.tax_total {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-14",
            path,
            format!("BT-110 {stated} ≠ Σ BT-117 {expected}"),
        )),
        None if !expected.is_zero() => report.push(Finding::fatal(
            "BR-CO-14",
            path,
            format!("BT-110 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_15(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(without) = totals.without_tax else {
        return;
    };
    let tax = totals
        .tax_total
        .unwrap_or(crate::amount::InvoiceAmount::ZERO);
    let Some(expected) = without.checked_add(tax) else {
        overflow(report, "BR-CO-15", 112, "BT-112");
        return;
    };
    match totals.with_tax {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-15",
            Path::group_term(Group::Totals, BtId(112)),
            format!("BT-112 {stated} ≠ BT-109 + BT-110 = {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-15",
            Path::group_term(Group::Totals, BtId(112)),
            format!("BT-112 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_16(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(with_tax) = totals.with_tax else {
        return;
    };
    let expected = match (totals.paid, totals.rounding) {
        (None, None) => Some(with_tax),
        (Some(p), None) => with_tax.checked_sub(p),
        (None, Some(r)) => with_tax.checked_add(r),
        (Some(p), Some(r)) => with_tax.checked_sub(p).and_then(|v| v.checked_add(r)),
    };
    let Some(expected) = expected else {
        overflow(report, "BR-CO-16", 115, "BT-115");
        return;
    };
    if totals.payable != expected {
        report.push(Finding::fatal(
            "BR-CO-16",
            Path::group_term(Group::Totals, BtId(115)),
            format!(
                "BT-115 {} ≠ BT-112 − BT-113 + BT-114 = {expected}",
                totals.payable
            ),
        ));
    }
}

fn br_co_17(invoice: &Invoice, report: &mut Report) {
    use crate::arith::{derived_vat, within_vat_tolerance, xpath_round};
    use rust_decimal::Decimal;
    for (i, e) in invoice.tax_breakdown.iter().enumerate() {
        if e.category.as_str().eq_ignore_ascii_case("TTX") {
            continue;
        }
        let path = Path::at_term(Group::TaxBreakdown, i, BtId(117));
        let rate = e.rate.map_or(Decimal::ZERO, Percentage::as_percent);
        if xpath_round(rate) == Decimal::ZERO {
            if xpath_round(e.tax.raw()) != Decimal::ZERO {
                report.push(Finding::fatal(
                    "BR-CO-17",
                    path,
                    format!("zero-rate group must have tax 0 (found {})", e.tax),
                ));
            }
            continue;
        }
        let Some(expected) = derived_vat(e.taxable.raw(), rate) else {
            continue;
        };
        let stated = e.tax.raw().abs();
        if !within_vat_tolerance(stated, expected) {
            report.push(Finding::fatal(
                "BR-CO-17",
                path,
                format!(
                    "BT-117 {} is not within ±1.00 exclusive of derived {expected}",
                    e.tax
                ),
            ));
        }
    }
}

pub static ALL: &[Rule] = &[
    Rule {
        id: "CORE-SPEC-01",
        severity: Severity::Fatal,
        text: "Unrecognised specification identifier (BT-24).",
        source: Source::Crate,
        eval: spec_lookup,
    },
    Rule {
        id: "CORE-PROCESS-01",
        severity: Severity::Fatal,
        text: "Self-billing (and other) process URNs are not validated as billing.",
        source: Source::Crate,
        eval: |_i, _r| {},
    },
    Rule {
        id: "IBR-SR-63",
        severity: Severity::Fatal,
        text: "BT-24 must not contain '*'.",
        source: Source::Crate,
        eval: |_i, _r| {},
    },
    Rule {
        id: "BR-01",
        severity: Severity::Fatal,
        text: "An Invoice shall have a Specification identifier (BT-24).",
        source: Source::Both,
        eval: br_01,
    },
    Rule {
        id: "BR-02",
        severity: Severity::Fatal,
        text: "Invoice number (BT-1) shall be present.",
        source: Source::Both,
        eval: br_02,
    },
    Rule {
        id: "BR-03",
        severity: Severity::Fatal,
        text: "An Invoice shall have an Invoice issue date (BT-2).",
        source: Source::Both,
        eval: br_03,
    },
    Rule {
        id: "BR-04",
        severity: Severity::Fatal,
        text: "An Invoice shall have an Invoice type code (BT-3).",
        source: Source::Both,
        eval: br_04,
    },
    Rule {
        id: "BR-09",
        severity: Severity::Fatal,
        text: "The Seller postal address shall contain a Seller country code (BT-40).",
        source: Source::Both,
        eval: br_09,
    },
    Rule {
        id: "BR-11",
        severity: Severity::Fatal,
        text: "The Buyer postal address shall contain a Buyer country code (BT-55).",
        source: Source::Both,
        eval: br_11,
    },
    Rule {
        id: "BR-21",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Invoice line identifier (BT-126).",
        source: Source::Both,
        eval: br_21,
    },
    Rule {
        id: "BR-25",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Item name (BT-153).",
        source: Source::Both,
        eval: br_25,
    },
    Rule {
        id: "BR-05",
        severity: Severity::Fatal,
        text: "Invoice currency code (BT-5) shall be present.",
        source: Source::Both,
        eval: br_05,
    },
    Rule {
        id: "BR-06",
        severity: Severity::Fatal,
        text: "Seller name (BT-27) shall be present.",
        source: Source::Both,
        eval: br_06,
    },
    Rule {
        id: "BR-07",
        severity: Severity::Fatal,
        text: "Buyer name (BT-44) shall be present.",
        source: Source::Both,
        eval: br_07,
    },
    Rule {
        id: "BR-16",
        severity: Severity::Fatal,
        text: "An Invoice shall have at least one Invoice line (BG-25).",
        source: Source::Both,
        eval: br_16,
    },
    Rule {
        id: "BR-CO-10",
        severity: Severity::Fatal,
        text: "Sum of Invoice line net amount (BT-106) = Σ Invoice line net amount (BT-131).",
        source: Source::Both,
        eval: br_co_10,
    },
    Rule {
        id: "BR-CO-11",
        severity: Severity::Fatal,
        text: "Sum of allowances on document level (BT-107) = Σ Document level allowance amount (BT-92). Absent if and only if there is no BG-20.",
        source: Source::Both,
        eval: br_co_11,
    },
    Rule {
        id: "BR-CO-12",
        severity: Severity::Fatal,
        text: "Sum of charges on document level (BT-108) = Σ Document level charge amount (BT-99). Absent if and only if there is no BG-21.",
        source: Source::Both,
        eval: br_co_12,
    },
    Rule {
        id: "BR-CO-13",
        severity: Severity::Fatal,
        text: "Invoice total amount without VAT (BT-109) = BT-106 − BT-107 + BT-108 (four presence branches; absent ≠ 0).",
        source: Source::Both,
        eval: br_co_13,
    },
    Rule {
        id: "BR-CO-14",
        severity: Severity::Fatal,
        text: "Invoice total tax amount (BT-110) = Σ tax category tax amount (BT-117). Exact. On PINT-MY, sum VAT-scheme rows only.",
        source: Source::Both,
        eval: br_co_14,
    },
    Rule {
        id: "BR-CO-15",
        severity: Severity::Fatal,
        text: "Invoice total amount with VAT (BT-112) = BT-109 + BT-110.",
        source: Source::Both,
        eval: br_co_15,
    },
    Rule {
        id: "BR-CO-16",
        severity: Severity::Fatal,
        text: "Amount due for payment (BT-115) = Invoice total amount with VAT (BT-112) − Paid amount (BT-113) + Rounding amount (BT-114).",
        source: Source::Both,
        eval: br_co_16,
    },
    Rule {
        id: "BR-CO-17",
        severity: Severity::Fatal,
        text: "VAT category tax amount (BT-117) = VAT category taxable amount (BT-116) × (VAT category rate (BT-119) / 100), rounded to two decimals. Artefact slack ±1.00 exclusive on abs; zero-rate branch has no slack.",
        source: Source::Both,
        eval: br_co_17,
    },
    Rule {
        id: "PINT-TAX",
        severity: Severity::Fatal,
        text: "Tax system on a line must be allowed by the profile. EN 16931 / Peppol BIS 3.0: VAT only. PINT / PINT-MY: VAT, GST, SST, consumption.",
        source: Source::Crate,
        eval: pint_tax,
    },
    Rule {
        id: "IBR-02-MY",
        severity: Severity::Fatal,
        text: "Seller legal registration identifier (BRN / IBT-030) shall be present.",
        source: Source::Crate,
        eval: ibr_my,
    },
    Rule {
        id: "IBR-03-MY",
        severity: Severity::Fatal,
        text: "Buyer legal registration identifier (BRN / IBT-047) shall be present.",
        source: Source::Crate,
        eval: |_i, _r| {},
    },
    Rule {
        id: "IBR-04-MY",
        severity: Severity::Fatal,
        text: "Seller TIN (IBT-032) shall be present.",
        source: Source::Crate,
        eval: |_i, _r| {},
    },
    Rule {
        id: "ALIGNED-IBRP-CL-01-MY",
        severity: Severity::Fatal,
        text: "Malaysian invoice tax categories shall be SA, SE, HVG, LVG, TTX, E or O.",
        source: Source::Crate,
        eval: |_i, _r| {},
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Code;

    #[test]
    fn padding_matches() {
        assert!(matches_id("BR-02", "br-2"));
        assert!(matches_id("BR-CO-16", "BR-CO-16"));
        assert!(explain("br-02").unwrap().contains("BT-1"));
        assert!(explain("nope").is_none());
        assert!(
            explain("BR-CO-16")
                .unwrap()
                .contains("BT-115) = Invoice total amount with VAT (BT-112)")
        );
        assert!(!explain("BR-CO-16").unwrap().contains("line net + tax"));
    }

    #[test]
    fn recargo_half_percent_does_not_take_zero_branch() {
        use crate::amount::InvoiceAmount;
        use crate::date::Date;
        use crate::invoice::{Invoice, Line, Party, TaxBreakdown};
        use crate::profile::Profile;
        use crate::tax::{TaxCategory, TaxSystem};
        use crate::validate;
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-R",
            "EUR",
            {
                let mut p = Party::new("S", "ES");
                p.vat_identifier = Some(crate::identifier::Identifier::new("ESA12345678"));
                p
            },
            Party::new("B", "ES"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        let rate = Percentage::new(Decimal::from_str("0.5").unwrap());
        inv.lines = vec![Line::new(
            "1",
            "Recargo",
            InvoiceAmount::parse("1000.00").unwrap(),
            TaxCategory::vat("S", rate),
        )];
        inv.tax_breakdown = vec![TaxBreakdown {
            system: TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("S"),
            rate: Some(rate),
            taxable: InvoiceAmount::parse("1000.00").unwrap(),
            tax: InvoiceAmount::parse("5.00").unwrap(),
            exemption_reason: None,
            exemption_code: None,
        }];
        crate::reconcile::reconcile(&mut inv).unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CO-17"),
            "{report}"
        );
    }
}
