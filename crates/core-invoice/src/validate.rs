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
    report.sort_stable();
    report
}
