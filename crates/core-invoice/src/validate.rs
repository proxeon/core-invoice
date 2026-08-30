use crate::invoice::Invoice;
use crate::report::Report;
use crate::rules;

/// Semantic checks on the in-memory invoice. Syntax (UBL/CII) lives in `core-invoice-formats`.
pub fn validate(invoice: &Invoice) -> Report {
    let mut report = Report {
        profile_slug: invoice.profile.slug(),
        rules_checked: rules::ALL.len(),
        ..Report::default()
    };
    for rule in rules::ALL {
        (rule.eval)(invoice, &mut report);
    }
    report.sort_stable();
    report
}
