//! Peppol BIS syntax-only walks. Model evals stay `syntax_or_option_pass`.
//! Not OpenPEPPOL Valid. Pin is `.sch` only.

use core_invoice::{BtId, Finding, Group, Invoice, Path, Profile, Report};

pub fn apply(xml: &str, invoice: &Invoice, report: &mut Report) {
    if invoice.profile != Profile::PeppolBis3 {
        return;
    }
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return;
    };
    let root = doc.root_element();
    match root.tag_name().name() {
        "Invoice" | "CreditNote" => walk_ubl(root, report),
        "CrossIndustryInvoice" => walk_cii(root, report),
        _ => {}
    }
}

fn local<'a>(n: roxmltree::Node<'a, 'a>) -> &'a str {
    n.tag_name().name()
}

fn is_elem(n: roxmltree::Node<'_, '_>) -> bool {
    n.is_element()
}

fn children<'a>(
    n: roxmltree::Node<'a, 'a>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    n.children()
        .filter(move |c| is_elem(*c) && local(*c) == name)
}

fn text_concat(n: roxmltree::Node<'_, '_>) -> String {
    n.children()
        .filter_map(|c| c.text())
        .collect::<Vec<_>>()
        .join("")
}

fn empty_element(n: roxmltree::Node<'_, '_>) -> bool {
    if !is_elem(n) {
        return false;
    }
    if n.children().any(is_elem) {
        return false;
    }
    text_concat(n).trim().is_empty()
}

fn walk_empty(n: roxmltree::Node<'_, '_>, report: &mut Report) {
    if empty_element(n) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R008",
            Path::group(Group::Document),
            "Document MUST not contain empty elements",
        ));
    }
    for c in n.children().filter(|c| is_elem(*c)) {
        walk_empty(c, report);
    }
}

fn charge_indicator_ok(s: &str) -> bool {
    matches!(s.trim(), "true" | "false")
}

fn walk_ubl(root: roxmltree::Node<'_, '_>, report: &mut Report) {
    walk_empty(root, report);
    walk_ubl_indicators(root, report);
    let with_sub = children(root, "TaxTotal")
        .filter(|t| children(*t, "TaxSubtotal").next().is_some())
        .count();
    if with_sub != 1 {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R053",
            Path::group(Group::TaxBreakdown),
            "Only one tax total with tax subtotals MUST be provided",
        ));
    }
    if local(root) == "CreditNote" {
        let n50 = children(root, "AdditionalDocumentReference")
            .filter(|n| {
                n.children()
                    .find(|c| is_elem(*c) && local(*c) == "DocumentTypeCode")
                    .and_then(|c| c.text())
                    .is_some_and(|t| t.trim() == "50")
            })
            .count();
        if n50 > 1 {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R080",
                Path::term(BtId(11)),
                "At most one project reference is allowed",
            ));
        }
    }
    for (i, line) in children(root, "InvoiceLine")
        .chain(children(root, "CreditNoteLine"))
        .enumerate()
    {
        if children(line, "DocumentReference").count() > 1 {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R100",
                Path::at_term(Group::Line, i, BtId(128)),
                "Only one invoiced object is allowed pr line",
            ));
        }
    }
}

fn walk_ubl_indicators(n: roxmltree::Node<'_, '_>, report: &mut Report) {
    if is_elem(n) && local(n) == "ChargeIndicator" {
        let t = text_concat(n);
        if !charge_indicator_ok(&t) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R043",
                Path::group(Group::DocumentAllowance),
                "ChargeIndicator must be true or false",
            ));
        }
        if let Some(ac) = n.parent()
            && local(ac) == "AllowanceCharge"
            && let Some(price) = ac.parent()
            && local(price) == "Price"
            && t.trim() != "false"
        {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R044",
                Path::term(BtId(147)),
                "Price-level AllowanceCharge ChargeIndicator must be false",
            ));
        }
    }
    for c in n.children().filter(|c| is_elem(*c)) {
        walk_ubl_indicators(c, report);
    }
}

fn walk_cii(root: roxmltree::Node<'_, '_>, report: &mut Report) {
    walk_cii_indicators(root, report);
    if let Some(st) = root
        .descendants()
        .find(|n| is_elem(*n) && local(*n) == "ApplicableHeaderTradeSettlement")
    {
        let doc_cur = st
            .children()
            .find(|n| is_elem(*n) && local(*n) == "InvoiceCurrencyCode")
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_owned();
        if let Some(sum) = st
            .children()
            .find(|n| is_elem(*n) && local(*n) == "SpecifiedTradeSettlementHeaderMonetarySummation")
        {
            let n = children(sum, "TaxTotalAmount")
                .filter(|a| a.attribute("currencyID").unwrap_or("").trim() == doc_cur)
                .count();
            if n != 1 {
                report.push(Finding::fatal(
                    "PEPPOL-EN16931-R053",
                    Path::group(Group::TaxBreakdown),
                    "Only one tax total with tax subtotals MUST be provided",
                ));
            }
        }
    }
    if let Some(ag) = root
        .descendants()
        .find(|n| is_elem(*n) && local(*n) == "ApplicableHeaderTradeAgreement")
    {
        let n130 = children(ag, "AdditionalReferencedDocument")
            .filter(|n| {
                n.children()
                    .find(|c| is_elem(*c) && local(*c) == "TypeCode")
                    .and_then(|c| c.text())
                    .is_some_and(|t| t.trim() == "130")
            })
            .count();
        if n130 > 1 {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R006",
                Path::term(BtId(18)),
                "Only one invoiced object is allowed on document level",
            ));
        }
    }
    for (i, line) in root
        .descendants()
        .filter(|n| is_elem(*n) && local(*n) == "IncludedSupplyChainTradeLineItem")
        .enumerate()
    {
        let n130 = line
            .descendants()
            .filter(|n| is_elem(*n) && local(*n) == "AdditionalReferencedDocument")
            .filter(|n| {
                n.children()
                    .find(|c| is_elem(*c) && local(*c) == "TypeCode")
                    .and_then(|c| c.text())
                    .is_some_and(|t| t.trim() == "130")
            })
            .count();
        if n130 > 1 {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R100",
                Path::at_term(Group::Line, i, BtId(128)),
                "Only one invoiced object is allowed pr line",
            ));
        }
    }
}

fn walk_cii_indicators(n: roxmltree::Node<'_, '_>, report: &mut Report) {
    if is_elem(n) && local(n) == "Indicator" {
        let t = text_concat(n);
        if !charge_indicator_ok(&t) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R043",
                Path::group(Group::DocumentAllowance),
                "ChargeIndicator must be true or false",
            ));
        }
    }
    for c in n.children().filter(|c| is_elem(*c)) {
        walk_cii_indicators(c, report);
    }
}
