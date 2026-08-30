use crate::invoice::Invoice;
use crate::report::Report;
use crate::rules;

/// Semantic checks on the in-memory invoice. Syntax (UBL/CII) lives in `core-invoice-formats`.
pub fn validate(invoice: &Invoice) -> Report {
    let rules = rules::catalogue();
    let mut report = Report {
        profile_slug: invoice.profile.slug(),
        rules_checked: rules.len(),
        ..Report::default()
    };
    for rule in rules {
        (rule.eval)(invoice, &mut report);
    }
    report.sort_stable();
    report
}
