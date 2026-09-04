//! Semantic checks on the in-memory invoice.

use crate::invoice::Invoice;
use crate::report::Report;
use crate::rules;

/// Semantic checks on the in-memory invoice. Syntax (UBL/CII) lives in `core-invoice-formats`.
pub fn validate(invoice: &Invoice) -> Report {
    let core = rules::core_rules();
    let extra = invoice.profile.extra_rules();
    let mut report = Report {
        profile_slug: invoice.profile.slug(),
        rules_checked: core.len() + extra.len(),
        ..Report::default()
    };
    for rule in core.iter().chain(extra) {
        (rule.eval)(invoice, &mut report);
    }
    #[cfg(feature = "xrechnung")]
    if crate::xrechnung::claimed(invoice) {
        for rule in crate::xrechnung::RULES {
            (rule.eval)(invoice, &mut report);
        }
        report.rules_checked += crate::xrechnung::RULES.len();
    }
    report.sort_stable();
    report
}

#[cfg(all(test, not(feature = "xrechnung")))]
mod tests {
    use super::*;
    use crate::invoice::{Invoice, Party};
    use crate::profile::Profile;

    #[test]
    fn xrechnung_claim_without_feature_does_not_emit_br_de() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "DE"),
        );
        inv.specification_id =
            Some("urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0".into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| !f.id.starts_with("BR-DE-") && f.id != "BR-TMP-2"),
            "{report}"
        );
    }
}
