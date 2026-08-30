use crate::invoice::Invoice;
use crate::report::{Finding, Report};

/// Semantic checks on the in-memory invoice. Syntax (UBL/CII) lives in `core-invoice-formats`.
pub fn validate(invoice: &Invoice) -> Report {
    let mut report = Report::default();

    if invoice.number.trim().is_empty() {
        report.push(Finding::fatal("BR-02", "Invoice number (BT-1) shall be present"));
    }

    if invoice.currency.trim().len() != 3 {
        report.push(Finding::fatal(
            "BR-05",
            "Invoice currency code (BT-5) shall be a 3-letter code",
        ));
    }

    if invoice.seller.name.trim().is_empty() {
        report.push(Finding::fatal("BR-06", "Seller name (BT-27) shall be present"));
    }

    if invoice.buyer.name.trim().is_empty() {
        report.push(Finding::fatal("BR-07", "Buyer name (BT-44) shall be present"));
    }

    if invoice.lines.is_empty() {
        report.push(Finding::fatal(
            "BR-16",
            "An invoice shall have at least one line (BG-25)",
        ));
    }

    for (i, line) in invoice.lines.iter().enumerate() {
        if !invoice.profile.allows(line.tax.system) {
            report.push(Finding::fatal(
                "PINT-TAX",
                format!(
                    "Line {} tax system {} is not allowed on profile {}",
                    i + 1,
                    line.tax.system.as_str(),
                    invoice.profile.slug()
                ),
            ));
        }
    }

    let net = invoice.line_net_sum();
    let expected_payable = net.saturating_add(invoice.tax_total);
    if expected_payable != invoice.payable {
        report.push(Finding::fatal(
            "BR-CO-16",
            format!(
                "Payable ({}) shall equal line net ({}) + tax total ({})",
                invoice.payable, net, invoice.tax_total
            ),
        ));
    }

    if invoice.profile == crate::profile::Profile::PintMy {
        match invoice.seller.id_scheme.as_deref() {
            Some("TIN") | Some("BRN") | Some("NRIC") | Some("PASSPORT") => {}
            Some(other) => report.push(Finding::fatal(
                "PINT-MY-ID",
                format!("Seller id scheme {other} is not a PINT-MY identification type"),
            )),
            None if invoice.seller.tax_id.is_some() => report.push(Finding::fatal(
                "PINT-MY-ID",
                "Seller tax id on PINT-MY requires a scheme (TIN, BRN, NRIC, PASSPORT)",
            )),
            None => {}
        }
    }

    report
}
