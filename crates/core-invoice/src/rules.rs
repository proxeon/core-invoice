use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::report::{Finding, Report, Severity, Source};

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
    ALL.iter().find(|r| matches_id(r.id, id)).map(|r| r.text)
}

pub fn catalogue() -> &'static [Rule] {
    ALL
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

fn br_co_16_unregistered(_invoice: &Invoice, _report: &mut Report) {}

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
        id: "BR-CO-16",
        severity: Severity::Fatal,
        text: "Amount due for payment (BT-115) = invoice total with tax (BT-112) − paid (BT-113) + rounding (BT-114). Not evaluated until document totals exist.",
        source: Source::Both,
        eval: br_co_16_unregistered,
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

    #[test]
    fn padding_matches() {
        assert!(matches_id("BR-02", "br-2"));
        assert!(matches_id("BR-CO-16", "BR-CO-16"));
        assert!(explain("br-02").unwrap().contains("BT-1"));
        assert!(explain("nope").is_none());
    }
}
